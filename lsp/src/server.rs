// Standard Uses
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::completion::completion;
use crate::diagnostics::{coverage, diagnostics, file};
use crate::document::{kind, DocumentKind};
use crate::hover::hover;
use crate::navigation::{definition, references, rename, symbol_at};
use crate::position::LineIndex;
use crate::preview::{preview, replacement_counts};
use crate::schema::LOCATIONS;
use crate::symbols::{folding, symbols};
use crate::syntax::Outline;
use crate::target;
use crate::toggle::toggle;
use crate::tokens::tokens;
use crate::trace::{Limits, Traces};

// External Uses
use codeshaper::core::registry::Registry;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics
};
use lsp_types::request::{
    ApplyWorkspaceEdit, CodeActionRequest, CodeLensRefresh, CodeLensRequest, Completion, DocumentSymbolRequest, ExecuteCommand, FoldingRangeRequest, GotoDefinition, HoverRequest,
    PrepareRenameRequest, References, Rename, Request as _, SemanticTokensFullRequest
};
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, ExecuteCommandParams,
    CodeLens, CodeLensParams, Command, CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, Location, MarkupContent, MarkupKind, Position, PublishDiagnosticsParams,
    Range, ReferenceParams, SemanticTokens, SemanticTokensParams, SemanticTokensResult, Uri,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRangeParams, PrepareRenameResponse, RenameParams,
    TextDocumentPositionParams, TextEdit, WorkspaceEdit
};
use serde_json::json;
use miette::{miette, IntoDiagnostic, Result};
use url::Url;


const PREVIEW_COMMAND: &str = "codeshaper.preview";
pub const TOGGLE_COMMAND: &str = "codeshaper.toggle";
const PREVIEW_REQUEST: &str = "codeshaper/preview";


#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    pub limits: Limits,
    pub refresh_lenses: bool
}

pub struct Server<'c> {
    connection: &'c Connection,
    registry: Registry,
    documents: HashMap<Uri, String>,
    traces: Traces,
    coverage: HashMap<Uri, Vec<lsp_types::Diagnostic>>,
    refresh_lenses: bool,
    next: i32
}

impl<'c> Server<'c> {
    pub fn new(connection: &'c Connection, registry: Registry, options: Options) -> Self {
        Self {
            connection, registry,
            documents: HashMap::new(),
            traces: Traces::new(options.limits),
            coverage: HashMap::new(),
            refresh_lenses: options.refresh_lenses,
            next: 0
        }
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
            HoverRequest::METHOD => {
                let params: HoverParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let patch = self.patch_at(&position.text_document.uri, position.position)
                    .map(|(project, text, offset)| (project, text.to_string(), offset));
                let found = patch.and_then(|(project, text, offset)| {
                    let (value, range) = hover(&self.registry, &project, &mut self.traces, &text, offset)?;
                    Some(Hover {
                        contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
                        range: Some(LineIndex::new(&text).range(range))
                    })
                });

                Response::new_ok(request.id, found)
            }
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri;
                let found = self.documents.get(&uri).and_then(|text| {
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);
                    let offset = index.offset(position.position);

                    if let Some(range) = definition(&outline, text, offset) {
                        return Some(GotoDefinitionResponse::Scalar(Location::new(uri.clone(), index.range(range))))
                    }

                    outline.literal_at(offset).filter(|(node, _)| outline.is_top(*node, "file"))?;
                    Some(GotoDefinitionResponse::Array(self.target_files(&uri, &outline)))
                });

                Response::new_ok(request.id, found)
            }
            References::METHOD => {
                let params: ReferenceParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position;
                let uri = position.text_document.uri;
                let found: Vec<Location> = self.documents.get(&uri).map(|text| {
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);
                    let offset = index.offset(position.position);

                    if let Some((_, literal)) = outline.literal_at(offset)
                        .filter(|(node, _)| LOCATIONS.contains(&outline.nodes[*node].name.as_str()))
                    {
                        return self.target_places(&uri, &outline, &literal.value)
                    }

                    references(&outline, text, offset, params.context.include_declaration).into_iter()
                        .map(|range| Location::new(uri.clone(), index.range(range)))
                        .collect()
                }).unwrap_or_default();

                Response::new_ok(request.id, found)
            }
            PrepareRenameRequest::METHOD => {
                let params: TextDocumentPositionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let found = self.documents.get(&params.text_document.uri).and_then(|text| {
                    let index = LineIndex::new(text);
                    let symbol = symbol_at(&Outline::parse(text), text, index.offset(params.position))?;
                    Some(PrepareRenameResponse::Range(index.range(symbol.range)))
                });

                Response::new_ok(request.id, found)
            }
            Rename::METHOD => {
                let params: RenameParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position;
                let uri = position.text_document.uri;
                let found = self.documents.get(&uri).and_then(|text| {
                    let index = LineIndex::new(text);
                    let ranges = rename(&Outline::parse(text), text, index.offset(position.position), &params.new_name)?;
                    let edits = ranges.into_iter().map(|range| TextEdit::new(index.range(range), params.new_name.clone())).collect();

                    Some(WorkspaceEdit::new(HashMap::from([(uri.clone(), edits)])))
                });

                Response::new_ok(request.id, found)
            }
            DocumentSymbolRequest::METHOD => {
                let params: DocumentSymbolParams = serde_json::from_value(request.params).into_diagnostic()?;
                let found = self.documents.get(&params.text_document.uri).map(|text| DocumentSymbolResponse::Nested(symbols(text)));

                Response::new_ok(request.id, found)
            }
            FoldingRangeRequest::METHOD => {
                let params: FoldingRangeParams = serde_json::from_value(request.params).into_diagnostic()?;
                let found = self.documents.get(&params.text_document.uri).map(|text| folding(text));

                Response::new_ok(request.id, found)
            }
            CodeLensRequest::METHOD => {
                let params: CodeLensParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                let command = |range, title: String, command: &str| CodeLens {
                    range,
                    command: Some(Command::new(title, command.to_string(), Some(vec![json!(uri.as_str())]))),
                    data: None
                };
                let lens = |range, title: String| command(range, title, PREVIEW_COMMAND);

                let mut lenses = vec![];
                if let Some((project, text, _)) = self.patch_at(&uri, Position::default()) {
                    lenses.push(lens(Range::default(), "Preview result".to_string()));

                    if let Some(toggle) = toggle(&Outline::parse(text)) {
                        lenses.push(command(LineIndex::new(text).range(toggle.node), toggle.title.to_string(), TOGGLE_COMMAND));
                    }

                    if let Some(trace) = self.traces.cached(&project, text) {
                        let index = LineIndex::new(text);
                        lenses.extend(replacement_counts(text, &trace).into_iter().map(|(range, title)| lens(index.range(range), title)));
                    }
                }

                Response::new_ok(request.id, lenses)
            }
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                let found: Vec<CodeActionOrCommand> = self.toggle(&uri)
                    .filter(|(node, _, _)| node.start.line <= params.range.end.line && params.range.start.line <= node.end.line)
                    .map(|(_, title, edit)| CodeActionOrCommand::CodeAction(CodeAction {
                        title: title.to_string(),
                        kind: Some(CodeActionKind::REFACTOR_REWRITE),
                        edit: Some(edit),
                        ..CodeAction::default()
                    }))
                    .into_iter().collect();

                Response::new_ok(request.id, found)
            }
            ExecuteCommand::METHOD => {
                let params: ExecuteCommandParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.arguments.first().and_then(|uri| uri.as_str()?.parse::<Uri>().ok());

                match uri.filter(|_| params.command == TOGGLE_COMMAND).and_then(|uri| self.toggle(&uri)) {
                    Some((_, title, edit)) => {
                        self.ask(ApplyWorkspaceEdit::METHOD, json!(ApplyWorkspaceEditParams { label: Some(title.to_string()), edit }))?;
                        Response::new_ok(request.id, json!(null))
                    }
                    None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch with an 'enabled' field".to_string())
                }
            }
            PREVIEW_REQUEST => self.preview(request),
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

    fn preview(&self, request: Request) -> Response {
        let uri = request.params.get("uri").and_then(|uri| uri.as_str()?.parse::<Uri>().ok());
        let Some((uri, (project, text, _))) = uri.and_then(|uri| Some((uri.clone(), self.patch_at(&uri, Position::default())?))) else {
            return Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch document".to_string())
        };
        let name = path(&uri).and_then(|path| Some(path.file_name()?.to_string_lossy().to_string())).unwrap_or_default();

        let (target, changes) = match preview(&self.registry, &project, &name, text) {
            Ok(previewed) => previewed,
            Err(error) => return Response::new_err(request.id, ErrorCode::RequestFailed as i32, message(&error))
        };

        let matched = changes.len();
        let mut files = vec![];
        let mut errors = vec![];

        for change in changes {
            let path = change.file.display().to_string();

            match change.result {
                Ok(result) if result != change.before => files.push(json!({
                    "uri": Url::from_file_path(target.join(&change.file)).map(|url| url.to_string()).unwrap_or_default(),
                    "path": path,
                    "before": change.before,
                    "result": result
                })),
                Ok(_) => {}
                Err(error) => errors.push(json!({ "path": path, "message": message(&error) }))
            }
        }

        Response::new_ok(request.id, json!({ "matched": matched, "files": files, "errors": errors }))
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
                self.documents.insert(params.text_document.uri.clone(), params.text_document.text);
                self.trace(&params.text_document.uri)?;
                self.publish(&params.text_document.uri)
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                let Some(change) = params.content_changes.into_iter().last() else { return Ok(()) };
                self.coverage.remove(&params.text_document.uri);
                self.update(params.text_document.uri, change.text)
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                if let Some(text) = params.text {
                    self.documents.insert(params.text_document.uri.clone(), text);
                }
                self.trace(&params.text_document.uri)?;
                self.publish(&params.text_document.uri)
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                self.documents.remove(&params.text_document.uri);
                self.coverage.remove(&params.text_document.uri);
                self.send(params.text_document.uri, vec![])
            }
            _ => Ok(())
        }
    }

    fn update(&mut self, uri: Uri, text: String) -> Result<()> {
        self.documents.insert(uri.clone(), text);
        self.publish(&uri)
    }

    fn trace(&mut self, uri: &Uri) -> Result<()> {
        let Some((project, text, _)) = self.patch_at(uri, Position::default()) else { return Ok(()) };
        let text = text.to_string();

        let mut found: Vec<lsp_types::Diagnostic> = file(&self.registry, &project, &text).into_iter().collect();
        let trace = self.traces.get(&self.registry, &project, &text);
        found.extend(trace.iter().flat_map(|trace| coverage(&text, trace)));
        self.coverage.insert(uri.clone(), found);

        if !self.refresh_lenses { return Ok(()) }

        self.ask(CodeLensRefresh::METHOD, json!(null))
    }

    fn ask(&mut self, method: &str, params: serde_json::Value) -> Result<()> {
        self.next += 1;
        let request = Request::new(RequestId::from(format!("server-{}", self.next)), method.to_string(), params);

        self.connection.sender.send(request.into()).into_diagnostic()
    }

    fn toggle(&self, uri: &Uri) -> Option<(Range, &'static str, WorkspaceEdit)> {
        let (_, text, _) = self.patch_at(uri, Position::default())?;
        let index = LineIndex::new(text);
        let toggle = toggle(&Outline::parse(text))?;
        let edit = TextEdit::new(index.range(toggle.value), toggle.text.to_string());

        Some((index.range(toggle.node), toggle.title, WorkspaceEdit::new(HashMap::from([(uri.clone(), vec![edit])]))))
    }

    fn target_files(&self, uri: &Uri, outline: &Outline) -> Vec<Location> {
        let Some((project, _, _)) = self.patch_at(uri, Position::default()) else { return vec![] };
        let Some((directory, files)) = target::files(&self.registry, &project, outline) else { return vec![] };

        files.iter()
            .filter_map(|file| Some(Location::new(file_uri(&directory.join(file))?, Range::default())))
            .collect()
    }

    fn target_places(&self, uri: &Uri, outline: &Outline, location: &str) -> Vec<Location> {
        let Some((project, _, _)) = self.patch_at(uri, Position::default()) else { return vec![] };
        let mut found = vec![];

        for places in target::places(&self.registry, &project, outline, location) {
            let Some(uri) = file_uri(&places.path) else { continue };
            let index = LineIndex::new(&places.source);

            found.extend(places.ranges.into_iter().map(|range| Location::new(uri.clone(), index.range(range))));
        }

        found
    }

    fn publish(&self, uri: &Uri) -> Result<()> {
        let Some(text) = self.documents.get(uri) else { return Ok(()) };
        let Some(path) = path(uri) else { return Ok(()) };

        let mut found = diagnostics(&self.registry, &path, text);
        found.extend(self.coverage.get(uri).into_iter().flatten().cloned());

        self.send(uri.clone(), found)
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

fn file_uri(path: &Path) -> Option<Uri> {
    Url::from_file_path(path).ok()?.as_str().parse().ok()
}

fn message(error: &miette::Report) -> String {
    error.chain().map(|cause| cause.to_string()).collect::<Vec<_>>().join(": ")
}
