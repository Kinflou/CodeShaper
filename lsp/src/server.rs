// Standard Uses
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// Crate Uses
use crate::completion::{completion, settings};
use crate::diagnostics::{diagnostics, redundant_alias};
use crate::document::{kind, DocumentKind};
use crate::groups::link_at;
use crate::inlay::{inlay_hints, Hints};
use crate::navigation::{definition, references, rename, symbol_at};
use crate::position::LineIndex;
use crate::preview::replacement_counts;
use crate::schema::LOCATIONS;
use crate::symbols::{folding, symbols};
use crate::syntax::Outline;
use crate::target::{self, forget_languages};
use crate::toggle::toggle;
use crate::tokens::tokens;
use crate::trace::Limits;
use crate::worker::{file_uri, path, Refresh, Shared, Snapshot, Work, Worker};

// External Uses
use codeshaper::core::registry::Registry;
use dunce::canonicalize;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _
};
use lsp_types::request::{
    ApplyWorkspaceEdit, CodeActionRequest, CodeLensRequest, Completion, DocumentHighlightRequest, DocumentSymbolRequest, ExecuteCommand, FoldingRangeRequest, GotoDefinition, HoverRequest, InlayHintRequest,
    PrepareRenameRequest, References, Rename, Request as _, SemanticTokensFullRequest
};
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, DocumentHighlight, DocumentHighlightParams, ExecuteCommandParams,
    CodeLens, CodeLensParams, Command, CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse,
    HoverParams, InlayHint, InlayHintParams, Location, LocationLink, Position,
    Range, ReferenceParams, SemanticTokens, SemanticTokensParams, SemanticTokensResult, Uri,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRangeParams, PrepareRenameResponse, RenameParams,
    TextDocumentPositionParams, TextEdit, WorkspaceEdit
};
use serde_json::json;
use miette::{miette, IntoDiagnostic, Result};


const PREVIEW_COMMAND: &str = "codeshaper.preview";
pub const TOGGLE_COMMAND: &str = "codeshaper.toggle";
const PREVIEW_REQUEST: &str = "codeshaper/preview";
const OPERATION_COMMAND: &str = "codeshaper.previewOperation";
const OPERATION_REQUEST: &str = "codeshaper/previewOperation";


#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    pub limits: Limits,
    pub refresh: Refresh,
    pub hints: Hints,
    pub definition_links: bool
}

pub struct Server<'c> {
    connection: &'c Connection,
    registry: Registry,
    documents: HashMap<Uri, String>,
    shared: Arc<Shared>,
    worker: Worker,
    definition_links: bool,
    hints: Hints
}

impl<'c> Server<'c> {
    pub fn new(connection: &'c Connection, registry: impl Fn() -> Registry + Send + 'static, options: Options) -> Self {
        let sender = connection.sender.clone();
        let shared = Arc::new(Shared::new(move |message| { let _ = sender.send(message); }, options.refresh, options.hints));
        let own = registry();
        let worker = Worker::start(shared.clone(), registry, options.limits);

        Self {
            connection,
            registry: own,
            documents: HashMap::new(),
            shared, worker,
            definition_links: options.definition_links,
            hints: options.hints
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
                let uri = position.text_document.uri;
                let items = match self.patch_at(&uri, position.position) {
                    Some((project, text, offset)) => completion(&self.registry, &project, text, offset),
                    None => self.documents.get(&uri).zip(path(&uri))
                        .filter(|(_, path)| kind(path) == DocumentKind::Settings)
                        .map(|(text, _)| settings(&self.registry, text, LineIndex::new(text).offset(position.position)))
                        .unwrap_or_default()
                };

                Response::new_ok(request.id, CompletionResponse::Array(items))
            }
            HoverRequest::METHOD => {
                let params: HoverParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri;
                let offset = self.documents.get(&uri).map(|text| LineIndex::new(text).offset(position.position)).unwrap_or_default();
                let work = self.snapshot(&uri).map(|snapshot| (snapshot, Work::Hover(offset)))
                    .or_else(|| Some((self.document(&uri, DocumentKind::Settings)?, Work::Languages(offset))));

                match work {
                    Some((snapshot, work)) => {
                        self.worker.request(request.id, snapshot, work);
                        return Ok(())
                    }
                    None => Response::new_ok(request.id, json!(null))
                }
            }
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri;
                let found = self.documents.get(&uri).and_then(|text| {
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);
                    let offset = index.offset(position.position);

                    let here = |range| vec![Location::new(uri.clone(), index.range(range))];

                    if let Some((symbol, range)) = symbol_at(&outline, text, offset).zip(definition(&outline, text, offset)) {
                        return Some(self.links(index.range(symbol.range), here(range)))
                    }

                    if let Some(link) = link_at(&outline, text, offset) {
                        let origin = link.ranges().into_iter().find(|range| range.start <= offset && offset <= range.end)?;
                        return Some(self.links(index.range(origin), here(link.group)))
                    }

                    let (_, literal) = outline.literal_at(offset).filter(|(node, _)| outline.is_top(*node, "file"))?;
                    Some(self.links(index.range(literal.content.clone()), self.target_files(&uri, &outline)))
                });

                Response::new_ok(request.id, found)
            }
            DocumentHighlightRequest::METHOD => {
                let params: DocumentHighlightParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let found: Option<Vec<DocumentHighlight>> = self.documents.get(&position.text_document.uri).and_then(|text| {
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);
                    let offset = index.offset(position.position);
                    let ranges = link_at(&outline, text, offset)
                        .map(|link| link.ranges())
                        .unwrap_or_else(|| references(&outline, text, offset, true));

                    (!ranges.is_empty()).then(|| ranges.into_iter()
                        .map(|range| DocumentHighlight { range: index.range(range), kind: None })
                        .collect())
                });

                Response::new_ok(request.id, found)
            }
            References::METHOD => {
                let params: ReferenceParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position;
                let uri = position.text_document.uri;

                let location = self.patch_at(&uri, position.position).and_then(|(_, text, offset)| {
                    let outline = Outline::parse(text);
                    let (node, literal) = outline.literal_at(offset)?;

                    LOCATIONS.contains(&outline.nodes[node].name.as_str()).then(|| literal.value.clone())
                });
                if let Some((snapshot, location)) = self.snapshot(&uri).zip(location) {
                    self.worker.request(request.id, snapshot, Work::Places(location));
                    return Ok(())
                }

                let found: Vec<Location> = self.documents.get(&uri).map(|text| {
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);
                    let offset = index.offset(position.position);

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
                let is_configuration = path(&uri).is_some_and(|path| kind(&path) == DocumentKind::Configuration);
                if let Some((_, text, _)) = self.patch_at(&uri, Position::default()) {
                    lenses.push(lens(Range::default(), "Preview result".to_string()));

                    if let Some(toggle) = toggle(&Outline::parse(text)) {
                        lenses.push(command(LineIndex::new(text).range(toggle.node), toggle.title.to_string(), TOGGLE_COMMAND));
                    }

                    if let Some(trace) = self.shared.trace(&uri) {
                        let index = LineIndex::new(text);
                        lenses.extend(replacement_counts(text, &trace).into_iter().map(|(range, title)| lens(index.range(range), title)));
                    }
                }
                if is_configuration || !lenses.is_empty() {
                    let place = usize::from(!lenses.is_empty());
                    lenses.insert(place, command(Range::default(), "Preview operation".to_string(), OPERATION_COMMAND));
                }

                Response::new_ok(request.id, lenses)
            }
            InlayHintRequest::METHOD => {
                let params: InlayHintParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                let found: Vec<InlayHint> = self.patch_at(&uri, Position::default())
                    .map(|(_, text, _)| inlay_hints(text, self.hints, &self.shared.counts(&uri)))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|hint| params.range.start <= hint.position && hint.position <= params.range.end)
                    .collect();

                Response::new_ok(request.id, found)
            }
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                let found: Vec<CodeActionOrCommand> = self.toggle(&uri).into_iter()
                    .chain(self.remove_alias(&uri))
                    .filter(|(node, _, _)| node.start.line <= params.range.end.line && params.range.start.line <= node.end.line)
                    .map(|(_, title, edit)| CodeActionOrCommand::CodeAction(CodeAction {
                        title: title.to_string(),
                        kind: Some(CodeActionKind::REFACTOR_REWRITE),
                        edit: Some(edit),
                        ..CodeAction::default()
                    }))
                    .collect();

                Response::new_ok(request.id, found)
            }
            ExecuteCommand::METHOD => {
                let params: ExecuteCommandParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.arguments.first().and_then(|uri| uri.as_str()?.parse::<Uri>().ok());

                match uri.filter(|_| params.command == TOGGLE_COMMAND).and_then(|uri| self.toggle(&uri)) {
                    Some((_, title, edit)) => {
                        self.shared.ask(ApplyWorkspaceEdit::METHOD, json!(ApplyWorkspaceEditParams { label: Some(title.to_string()), edit }));
                        Response::new_ok(request.id, json!(null))
                    }
                    None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch with an 'enabled' field".to_string())
                }
            }
            PREVIEW_REQUEST => {
                let uri = request.params.get("uri").and_then(|uri| uri.as_str()?.parse::<Uri>().ok());

                match uri.and_then(|uri| self.snapshot(&uri)) {
                    Some(snapshot) => {
                        self.worker.request(request.id, snapshot, Work::Preview);
                        return Ok(())
                    }
                    None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch document".to_string())
                }
            }
            OPERATION_REQUEST => {
                let uri = request.params.get("uri").and_then(|uri| uri.as_str()?.parse::<Uri>().ok());
                let found = uri.and_then(|uri| {
                    let path = path(&uri)?;
                    let configuration = (kind(&path) == DocumentKind::Configuration).then_some(path);

                    match &configuration {
                        Some(_) => Some((Snapshot {
                            uri: uri.clone(),
                            project: PathBuf::new(),
                            text: self.documents.get(&uri)?.clone(),
                            revision: self.shared.revision(&uri)?
                        }, configuration)),
                        None => Some((self.snapshot(&uri)?, None))
                    }
                });

                match found {
                    Some((snapshot, configuration)) => {
                        let open = self.documents.iter()
                            .filter_map(|(uri, text)| Some((canonicalize(path(uri)?).ok()?, text.clone())))
                            .collect();
                        self.worker.request(request.id, snapshot, Work::Operation { configuration, open });
                        return Ok(())
                    }
                    None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch or configuration".to_string())
                }
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

    fn snapshot(&self, uri: &Uri) -> Option<Snapshot> {
        let (project, text, _) = self.patch_at(uri, Position::default())?;

        Some(Snapshot { uri: uri.clone(), project, text: text.to_string(), revision: self.shared.revision(uri)? })
    }

    fn document(&self, uri: &Uri, wanted: DocumentKind) -> Option<Snapshot> {
        let path = path(uri).filter(|path| kind(path) == wanted)?;

        Some(Snapshot {
            uri: uri.clone(),
            project: path.parent()?.to_path_buf(),
            text: self.documents.get(uri)?.clone(),
            revision: self.shared.revision(uri)?
        })
    }

    fn notification(&mut self, notification: Notification) -> Result<()> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                self.documents.insert(uri.clone(), params.text_document.text);
                forget_languages();
                self.update(&uri, true);
                self.trace(&uri);
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                let Some(change) = params.content_changes.into_iter().last() else { return Ok(()) };
                let uri = params.text_document.uri;
                self.documents.insert(uri.clone(), change.text);
                self.update(&uri, true);
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                forget_languages();
                let changed = params.text.is_some_and(|text| self.documents.insert(uri.clone(), text.clone()) != Some(text));
                self.update(&uri, changed);
                self.trace(&uri);

                let project = path(&uri).filter(|path| kind(path) == DocumentKind::Settings).and_then(|path| path.parent().map(PathBuf::from));
                if let Some(project) = project {
                    let patches: Vec<Uri> = self.documents.keys()
                        .filter(|uri| self.patch_at(uri, Position::default()).is_some_and(|(found, _, _)| found == project))
                        .cloned().collect();

                    for uri in patches {
                        self.update(&uri, false);
                        self.trace(&uri);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                self.documents.remove(&params.text_document.uri);
                self.shared.close(&params.text_document.uri);
            }
            Cancel::METHOD => {
                if let Some(id) = notification.params.get("id").and_then(|id| serde_json::from_value::<RequestId>(id.clone()).ok()) {
                    self.shared.cancel(id);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn update(&self, uri: &Uri, changed: bool) {
        let Some((text, path)) = self.documents.get(uri).zip(path(uri)) else { return };

        self.shared.update(uri, diagnostics(&self.registry, &path, text), changed);
    }

    fn trace(&self, uri: &Uri) {
        if let Some(snapshot) = self.snapshot(uri) {
            self.worker.trace(snapshot);
        }
    }

    fn toggle(&self, uri: &Uri) -> Option<(Range, &'static str, WorkspaceEdit)> {
        let (_, text, _) = self.patch_at(uri, Position::default())?;
        let index = LineIndex::new(text);
        let toggle = toggle(&Outline::parse(text))?;
        let edit = TextEdit::new(index.range(toggle.value), toggle.text.to_string());

        Some((index.range(toggle.node), toggle.title, WorkspaceEdit::new(HashMap::from([(uri.clone(), vec![edit])]))))
    }

    fn links(&self, origin: Range, locations: Vec<Location>) -> GotoDefinitionResponse {
        if !self.definition_links { return GotoDefinitionResponse::Array(locations) }

        GotoDefinitionResponse::Link(locations.into_iter()
            .map(|location| LocationLink {
                origin_selection_range: Some(origin),
                target_uri: location.uri,
                target_range: location.range,
                target_selection_range: location.range
            })
            .collect())
    }

    fn remove_alias(&self, uri: &Uri) -> Option<(Range, &'static str, WorkspaceEdit)> {
        let (project, text, _) = self.patch_at(uri, Position::default())?;
        let alias = redundant_alias(&project, text)?;
        let index = LineIndex::new(text);
        let start = text[..alias.start].rfind('\n').map(|line| line + 1).unwrap_or(0);
        let end = text[alias.end..].find('\n').map(|line| alias.end + line + 1).unwrap_or(text.len());
        let whole = text[start..alias.start].trim().is_empty() && text[alias.end..end].trim().is_empty();
        let separator = text[alias.end..].len() - text[alias.end..].trim_start_matches([' ', '\t']).trim_start_matches(';').len();
        let removed = if whole { start..end } else { alias.start..alias.end + separator };
        let edit = TextEdit::new(index.range(removed), String::new());

        Some((index.range(alias), "Remove alias", WorkspaceEdit::new(HashMap::from([(uri.clone(), vec![edit])]))))
    }

    fn target_files(&self, uri: &Uri, outline: &Outline) -> Vec<Location> {
        let Some((project, _, _)) = self.patch_at(uri, Position::default()) else { return vec![] };
        let Some((directory, files)) = target::files(&self.registry, &project, outline) else { return vec![] };

        files.iter()
            .filter_map(|file| Some(Location::new(file_uri(&directory.join(file))?, Range::default())))
            .collect()
    }
}
