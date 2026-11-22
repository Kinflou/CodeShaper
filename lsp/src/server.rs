// Standard Uses
use std::collections::HashMap;

// Crate Uses
use crate::diagnostics::diagnostics;

// External Uses
use codeshaper::core::registry::Registry;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, PublishDiagnosticsParams, Uri
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
        let message = format!("Unsupported request '{}'", request.method);
        let response = Response::new_err(request.id, ErrorCode::MethodNotFound as i32, message);

        self.connection.sender.send(response.into()).into_diagnostic()
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
        let Some(path) = Url::parse(uri.as_str()).ok().and_then(|url| url.to_file_path().ok()) else {
            return Ok(())
        };

        self.send(uri.clone(), diagnostics(&self.registry, &path, text))
    }

    fn send(&self, uri: Uri, diagnostics: Vec<lsp_types::Diagnostic>) -> Result<()> {
        let params = PublishDiagnosticsParams { uri, diagnostics, version: None };
        let notification = Notification::new(PublishDiagnostics::METHOD.to_string(), params);

        self.connection.sender.send(notification.into()).into_diagnostic()
    }
}

