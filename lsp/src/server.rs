// Standard Uses
use std::collections::HashMap;
use std::path::PathBuf;

// Crate Uses
use crate::completion::completion;
use crate::diagnostics::diagnostics;
use crate::document::{kind, DocumentKind};
use crate::navigation::{definition, references};
use crate::position::LineIndex;
use crate::syntax::Outline;
use crate::tokens::tokens;

// External Uses
use codeshaper::core::registry::Registry;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics
};
use lsp_types::request::{Completion, GotoDefinition, References, Request as _, SemanticTokensFullRequest};
use lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Location,
    Position, PublishDiagnosticsParams, ReferenceParams, SemanticTokens, SemanticTokensParams, SemanticTokensResult,
    Uri
};
use miette::{miette, IntoDiagnostic, Result};
use url::Url;


pub struct Server<'c> {
    connection: &'c Connection,
    registry: Registry,
    documents: HashMap<Uri, String>
}

impl<'c> Server<'c> {
    pub fn new(connection: &'c Connection, registry: Registry) -> Self {
        Self { connection, registry, documents: HashMap::new() }
    }

    pub fn serve(&mut self) -> Result<()> {
        for message in &self.connection.receiver {
            match message {
                Message::Request(request) => {
                    if self.connection.handle_shutdown(&request).map_err(|error| miette!("{error}"))? {
                        return Ok(())
                    }
                    self.request(request)?;
                }
                Message::Notification(notification) => self.notification(notification)?,
                Message::Response(_) => {}
            }
        }

        Ok(())
    }

    fn request(&mut self, request: Request) -> Result<()> {
        let response = match request.method.as_str() {
            Completion::METHOD => {
                let params: CompletionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position;
                let items = self.patch_at(&position.text_document.uri, position.position)
                    .map(|(project, text, offset)| completion(&self.registry, &project, text, offset))
                    .unwrap_or_default();

                Response::new_ok(request.id, CompletionResponse::Array(items))
            }
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri;
                let found = self.documents.get(&uri).and_then(|text| {
                    let index = LineIndex::new(text);
                    let range = definition(&Outline::parse(text), text, index.offset(position.position))?;
                    Some(GotoDefinitionResponse::Scalar(Location::new(uri.clone(), index.range(range))))
                });

                Response::new_ok(request.id, found)
            }
            References::METHOD => {
                let params: ReferenceParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position;
                let uri = position.text_document.uri;
                let found: Vec<Location> = self.documents.get(&uri).map(|text| {
                    let index = LineIndex::new(text);
                    let offset = index.offset(position.position);

                    references(&Outline::parse(text), text, offset, params.context.include_declaration).into_iter()
                        .map(|range| Location::new(uri.clone(), index.range(range)))
                        .collect()
                }).unwrap_or_default();

                Response::new_ok(request.id, found)
            }
            SemanticTokensFullRequest::METHOD => {
                let params: SemanticTokensParams = serde_json::from_value(request.params).into_diagnostic()?;
                let data = self.documents.get(&params.text_document.uri).map(|text| tokens(text)).unwrap_or_default();

                Response::new_ok(request.id, SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data }))
            }
            method => {
                let message = format!("Unsupported request '{method}'");
                Response::new_err(request.id, ErrorCode::MethodNotFound as i32, message)
            }
        };

        self.connection.sender.send(response.into()).into_diagnostic()
    }

    fn patch_at(&self, uri: &Uri, position: Position) -> Option<(PathBuf, &str, usize)> {
        let text = self.documents.get(uri)?;
        let DocumentKind::Patch { project } = kind(&path(uri)?) else { return None };

        Some((project, text.as_str(), LineIndex::new(text).offset(position)))
    }

    fn notification(&mut self, notification: Notification) -> Result<()> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                self.update(params.text_document.uri, params.text_document.text)
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                let Some(change) = params.content_changes.into_iter().last() else { return Ok(()) };
                self.update(params.text_document.uri, change.text)
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                match params.text {
                    Some(text) => self.update(params.text_document.uri, text),
                    None => self.publish(&params.text_document.uri)
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                self.documents.remove(&params.text_document.uri);
                self.send(params.text_document.uri, vec![])
            }
            _ => Ok(())
        }
    }

    fn update(&mut self, uri: Uri, text: String) -> Result<()> {
        self.documents.insert(uri.clone(), text);
        self.publish(&uri)
    }

    fn publish(&self, uri: &Uri) -> Result<()> {
        let Some(text) = self.documents.get(uri) else { return Ok(()) };
        let Some(path) = path(uri) else { return Ok(()) };

        self.send(uri.clone(), diagnostics(&self.registry, &path, text))
    }

    fn send(&self, uri: Uri, diagnostics: Vec<lsp_types::Diagnostic>) -> Result<()> {
        let params = PublishDiagnosticsParams { uri, diagnostics, version: None };
        let notification = Notification::new(PublishDiagnostics::METHOD.to_string(), params);

        self.connection.sender.send(notification.into()).into_diagnostic()
    }
}


fn path(uri: &Uri) -> Option<PathBuf> {
    Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}
