// Standard Uses
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Crate Uses
use crate::completion::{completion, document};
use crate::diagnostics::{diagnostics, names, redundant_alias};
use crate::document::{kind, DocumentKind};
use crate::explore;
use crate::groups::link_at;
use crate::inlay::{inlay_hints, Hints};
use crate::navigation::{definition, links, references, rename, symbol_at};
use crate::options;
use crate::paths::{self, Opening};
use crate::position::LineIndex;
use crate::preview::replacement_counts;
use crate::schema::LOCATIONS;
use crate::section::{view_at, views, View};
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
    Cancel, DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _
};
use lsp_types::request::{
    ApplyWorkspaceEdit, CodeActionRequest, CodeLensRequest, Completion, DocumentHighlightRequest, DocumentSymbolRequest, ExecuteCommand, FoldingRangeRequest, GotoDefinition, HoverRequest, InlayHintRequest,
    PrepareRenameRequest, References, RegisterCapability, Rename, Request as _, SemanticTokensFullRequest
};
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, DocumentHighlight, DocumentHighlightParams, ExecuteCommandParams,
    CodeLens, CodeLensParams, Command, CompletionItemKind, CompletionParams, DidChangeWatchedFilesParams, DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams,
    HoverParams, InlayHint, InlayHintParams, Location, Position,
    Range, ReferenceParams, Registration, RegistrationParams, WatchKind, SemanticTokens, SemanticTokensParams, SemanticTokensResult, Uri,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRangeParams, PrepareRenameResponse, RenameParams,
    TextDocumentPositionParams, TextEdit, WorkspaceEdit
};
use serde_json::json;
use miette::{miette, IntoDiagnostic, Result};


const PREVIEW_COMMAND: &str = "codeshaper.preview";
const SUGGEST_COMMAND: &str = "editor.action.triggerSuggest";
pub const TOGGLE_COMMAND: &str = "codeshaper.toggle";
const PREVIEW_REQUEST: &str = "codeshaper/preview";
const OPERATION_COMMAND: &str = "codeshaper.previewOperation";
const OPERATION_REQUEST: &str = "codeshaper/previewOperation";
const EXPLORE_COMMAND: &str = "codeshaper.explore";


#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    pub limits: Limits,
    pub refresh: Refresh,
    pub hints: Hints,
    pub definition_links: bool,
    pub suggest_again: bool,
    pub watch: bool
}

pub struct Server<'c> {
    connection: &'c Connection,
    registry: Registry,
    documents: HashMap<Uri, String>,
    names: HashMap<Uri, Vec<String>>,
    shared: Arc<Shared>,
    worker: Worker,
    definition_links: bool,
    suggest_again: bool,
    watch: bool,
    hints: Hints
}

type Edit = (Range, &'static str, WorkspaceEdit);

enum Chosen {
    One(Option<String>, String),
    Several(Vec<String>),
    Unknown(String)
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
            names: HashMap::new(),
            shared, worker,
            definition_links: options.definition_links,
            suggest_again: options.suggest_again,
            watch: options.watch,
            hints: options.hints
        }
    }

    pub fn serve(&mut self) -> Result<()> {
        if self.watch {
            let watcher = FileSystemWatcher { glob_pattern: GlobPattern::String("**/*".to_string()), kind: Some(WatchKind::Create | WatchKind::Delete) };
            let registration = Registration {
                id: "files".to_string(),
                method: DidChangeWatchedFiles::METHOD.to_string(),
                register_options: Some(json!(DidChangeWatchedFilesRegistrationOptions { watchers: vec![watcher] }))
            };
            self.shared.ask(RegisterCapability::METHOD, json!(RegistrationParams { registrations: vec![registration] }));
        }

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
                let mut items = match self.patch(&uri) {
                    Some((project, text)) => completion(&self.registry, &project, text, LineIndex::new(text).offset(position.position)),
                    None => self.documents.get(&uri).zip(path(&uri))
                        .map(|(text, path)| (text, LineIndex::new(text).offset(position.position), path))
                        .map(|(text, offset, path)| document(&self.registry, &path, text, offset))
                        .unwrap_or_default()
                };
                if self.suggest_again {
                    for item in items.iter_mut().filter(|item| item.kind == Some(CompletionItemKind::FOLDER)) {
                        item.command = Some(Command::new("Suggest".to_string(), SUGGEST_COMMAND.to_string(), None));
                    }
                }

                Response::new_ok(request.id, CompletionResponse::Array(items))
            }
            HoverRequest::METHOD => {
                let params: HoverParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri;
                let offset = self.documents.get(&uri).map(|text| LineIndex::new(text).offset(position.position)).unwrap_or_default();
                let work = self.snapshot(&uri).map(|snapshot| (snapshot, Work::Hover(offset)))
                    .or_else(|| Some((self.listing(&uri)?, Work::Explain(offset))));

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
                let opening = self.documents.get(&uri).zip(path(&uri)).and_then(|(text, path)| {
                    let index = LineIndex::new(text);
                    let (range, opening) = paths::opening(&path, text, index.offset(position.position))?;

                    Some((index.range(range), opening))
                });
                match opening {
                    Some((origin, Opening::Pattern(field, pattern))) => {
                        if let Some(snapshot) = self.listing(&uri) {
                            self.worker.request(request.id, snapshot, Work::Open { field, pattern, origin, links: self.definition_links });
                            return Ok(())
                        }
                    }
                    Some((origin, Opening::File(file))) => {
                        let found = file_uri(&file).map(|file| links(self.definition_links, origin, vec![Location::new(file, Range::default())]));
                        return self.connection.sender.send(Response::new_ok(request.id, found).into()).into_diagnostic()
                    }
                    None => {}
                }

                let found = self.view(&uri, position.position).and_then(|(view, offset)| {
                    let text = view.text.as_ref();
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);

                    let here = |range| vec![Location::new(uri.clone(), index.range(range))];

                    if let Some((symbol, range)) = symbol_at(&outline, text, offset).zip(definition(&outline, text, offset)) {
                        return Some(links(self.definition_links, index.range(symbol.range), here(range)))
                    }

                    if let Some(link) = link_at(&outline, text, offset) {
                        let origin = link.ranges().into_iter().find(|range| range.start <= offset && offset <= range.end)?;
                        return Some(links(self.definition_links, index.range(origin), here(link.group)))
                    }

                    let (_, literal) = outline.literal_at(offset).filter(|(node, _)| outline.is_top(*node, "file"))?;
                    Some(links(self.definition_links, index.range(literal.content.clone()), self.target_files(&uri, &outline)))
                });

                Response::new_ok(request.id, found)
            }
            DocumentHighlightRequest::METHOD => {
                let params: DocumentHighlightParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position_params;
                let found: Option<Vec<DocumentHighlight>> = self.view(&position.text_document.uri, position.position).and_then(|(view, offset)| {
                    let text = view.text.as_ref();
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);
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

                let view = self.view(&uri, position.position);

                let location = view.as_ref().and_then(|(view, offset)| {
                    let outline = Outline::parse(&view.text);
                    let (node, literal) = outline.literal_at(*offset)?;

                    LOCATIONS.contains(&outline.nodes[node].name.as_str()).then(|| (view.text.to_string(), literal.value.clone()))
                });
                if let Some((snapshot, (text, location))) = self.snapshot(&uri).zip(location) {
                    self.worker.request(request.id, Snapshot { text, ..snapshot }, Work::Places(location));
                    return Ok(())
                }

                let found: Vec<Location> = view.map(|(view, offset)| {
                    let text = view.text.as_ref();
                    let index = LineIndex::new(text);
                    let outline = Outline::parse(text);

                    references(&outline, text, offset, params.context.include_declaration).into_iter()
                        .map(|range| Location::new(uri.clone(), index.range(range)))
                        .collect()
                }).unwrap_or_default();

                Response::new_ok(request.id, found)
            }
            PrepareRenameRequest::METHOD => {
                let params: TextDocumentPositionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let found = self.view(&params.text_document.uri, params.position).and_then(|(view, offset)| {
                    let symbol = symbol_at(&Outline::parse(&view.text), &view.text, offset)?;
                    Some(PrepareRenameResponse::Range(LineIndex::new(&view.text).range(symbol.range)))
                });

                Response::new_ok(request.id, found)
            }
            Rename::METHOD => {
                let params: RenameParams = serde_json::from_value(request.params).into_diagnostic()?;
                let position = params.text_document_position;
                let uri = position.text_document.uri;
                let found = self.view(&uri, position.position).and_then(|(view, offset)| {
                    let index = LineIndex::new(&view.text);
                    let ranges = rename(&Outline::parse(&view.text), &view.text, offset, &params.new_name)?;
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
                let lens = |range, title: &str, command: &str, name: Option<&str>| {
                    let arguments = std::iter::once(uri.as_str()).chain(name).map(|argument| json!(argument)).collect();

                    CodeLens { range, command: Some(Command::new(title.to_string(), command.to_string(), Some(arguments))), data: None }
                };
                let operation = lens(Range::default(), "Preview operation", OPERATION_COMMAND, None);
                let explored = |range, name| lens(range, "Explore target", EXPLORE_COMMAND, name);
                let is_configuration = path(&uri).is_some_and(|path| kind(&path) == DocumentKind::Configuration);

                let mut lenses = vec![];
                if let Some((_, text)) = self.patch(&uri) {
                    let index = LineIndex::new(text);
                    let traces = self.shared.traces(&uri);

                    for (number, view) in views(text).iter().enumerate() {
                        let name = view.name();
                        let place = view.section.as_ref().map(|section| index.range(section.whole.start..section.named.end)).unwrap_or_default();
                        let previewed = lens(place, "Preview result", PREVIEW_COMMAND, name);

                        match &view.section {
                            Some(_) if number == 0 => lenses.extend([operation.clone(), previewed]),
                            Some(_) => lenses.push(previewed),
                            None => lenses.extend([previewed, operation.clone()])
                        }
                        lenses.push(explored(place, name));

                        if let Some(toggle) = toggle(&Outline::parse(&view.text)) {
                            lenses.push(lens(index.range(toggle.node), toggle.title, TOGGLE_COMMAND, name));
                        }
                        if let Some(trace) = traces.get(number).cloned().flatten() {
                            lenses.extend(replacement_counts(&view.text, &trace).into_iter()
                                .map(|(range, title)| lens(index.range(range), &title, PREVIEW_COMMAND, name)));
                        }
                    }
                } else if is_configuration {
                    lenses.extend([operation, explored(Range::default(), None)]);
                }

                Response::new_ok(request.id, lenses)
            }
            InlayHintRequest::METHOD => {
                let params: InlayHintParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                let found: Vec<InlayHint> = self.patch(&uri)
                    .map(|(_, text)| inlay_hints(text, self.hints, &self.shared.counts(&uri)))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|hint| params.range.start <= hint.position && hint.position <= params.range.end)
                    .collect();

                Response::new_ok(request.id, found)
            }
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(request.params).into_diagnostic()?;
                let uri = params.text_document.uri;
                let found: Vec<CodeActionOrCommand> = self.patch(&uri).into_iter()
                    .flat_map(|(project, text)| views(text).into_iter()
                        .flat_map(|view| [toggled(&uri, &view), remove_alias(&uri, &project, text, &view)])
                        .collect::<Vec<_>>())
                    .flatten()
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
                let name = params.arguments.get(1).and_then(|name| name.as_str());

                match uri.filter(|_| params.command == TOGGLE_COMMAND).and_then(|uri| self.toggle(&uri, name)) {
                    Some((_, title, edit)) => {
                        self.shared.ask(ApplyWorkspaceEdit::METHOD, json!(ApplyWorkspaceEditParams { label: Some(title.to_string()), edit }));
                        Response::new_ok(request.id, json!(null))
                    }
                    None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch with an 'enabled' field".to_string())
                }
            }
            PREVIEW_REQUEST => {
                let uri = request.params.get("uri").and_then(|uri| uri.as_str()?.parse::<Uri>().ok());
                let name = request.params.get("name").and_then(|name| name.as_str());
                let position = request.params.get("position").and_then(|position| serde_json::from_value(position.clone()).ok());

                match uri.and_then(|uri| Some((self.snapshot(&uri)?, self.chosen(&uri, name, position)?))) {
                    Some((snapshot, Chosen::One(name, text))) => {
                        self.worker.request(request.id, Snapshot { text, ..snapshot }, Work::Preview(name));
                        return Ok(())
                    }
                    Some((_, Chosen::Several(names))) => Response::new_ok(request.id, json!({ "patches": names })),
                    Some((_, Chosen::Unknown(name))) => {
                        Response::new_err(request.id, ErrorCode::InvalidParams as i32, format!("The file has no patch named '{name}'"))
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
                        self.worker.request(request.id, snapshot, Work::Operation { configuration, open: self.open() });
                        return Ok(())
                    }
                    None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not an open patch or configuration".to_string())
                }
            }
            method if explore::REQUESTS.contains(&method) => match explore::start(&request.params) {
                Some((uri, start)) => {
                    let snapshot = Snapshot { uri, project: PathBuf::new(), text: String::new(), revision: 0 };
                    let work = Work::Explore { method: request.method, start, params: request.params, open: self.open() };
                    self.worker.request(request.id, snapshot, work);
                    return Ok(())
                }
                None => Response::new_err(request.id, ErrorCode::InvalidParams as i32, "Not a patch or configuration".to_string())
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

    fn open(&self) -> HashMap<PathBuf, String> {
        self.documents.iter()
            .filter_map(|(uri, text)| Some((canonicalize(path(uri)?).ok()?, text.clone())))
            .collect()
    }

    fn patch(&self, uri: &Uri) -> Option<(PathBuf, &str)> {
        let text = self.documents.get(uri)?;
        let DocumentKind::Patch { project } = kind(&path(uri)?) else { return None };

        Some((project, text.as_str()))
    }

    fn view(&self, uri: &Uri, position: Position) -> Option<(View<'_>, usize)> {
        let text = self.documents.get(uri)?;
        let offset = LineIndex::new(text).offset(position);
        let view = match self.patch(uri) {
            Some(_) => view_at(text, offset),
            None => View { section: None, text: Cow::Borrowed(text) }
        };

        Some((view, offset))
    }

    fn chosen(&self, uri: &Uri, name: Option<&str>, position: Option<Position>) -> Option<Chosen> {
        let (_, text) = self.patch(uri)?;
        let mut views = views(text);
        let offset = position.map(|position| LineIndex::new(text).offset(position));
        let holds = |view: &View| view.section.as_ref().zip(offset)
            .is_some_and(|(section, offset)| section.whole.start <= offset && offset <= section.whole.end);

        let index = match name {
            Some(name) => match views.iter().position(|view| view.name() == Some(name)) {
                None => return Some(Chosen::Unknown(name.to_string())),
                index => index
            },
            None if views.len() == 1 => Some(0),
            None => views.iter().position(holds)
        };
        let Some(index) = index else {
            return Some(Chosen::Several(views.iter().filter_map(|view| Some(view.name()?.to_string())).collect()))
        };
        let view = views.swap_remove(index);

        Some(Chosen::One(view.name().map(str::to_string), view.text.into_owned()))
    }

    fn snapshot(&self, uri: &Uri) -> Option<Snapshot> {
        let (project, text) = self.patch(uri)?;

        Some(Snapshot { uri: uri.clone(), project, text: text.to_string(), revision: self.shared.revision(uri)? })
    }

    fn listing(&self, uri: &Uri) -> Option<Snapshot> {
        let path = path(uri).filter(|path| matches!(kind(path), DocumentKind::Settings | DocumentKind::Configuration))?;

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
                self.remember_names(&uri);
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

                let renamed = self.remember_names(&uri);
                if let Some(project) = self.project(&uri).filter(|_| renamed || self.listing(&uri).is_some()) {
                    let others: Vec<Uri> = self.documents.keys()
                        .filter(|other| **other != uri && self.project(other).as_ref() == Some(&project))
                        .cloned().collect();

                    for uri in others {
                        self.update(&uri, false);
                        self.trace(&uri);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params).into_diagnostic()?;
                self.documents.remove(&params.text_document.uri);
                self.names.remove(&params.text_document.uri);
                self.shared.close(&params.text_document.uri);
            }
            Cancel::METHOD => {
                if let Some(id) = notification.params.get("id").and_then(|id| serde_json::from_value::<RequestId>(id.clone()).ok()) {
                    self.shared.cancel(id);
                }
            }
            DidChangeWatchedFiles::METHOD => {
                let params: DidChangeWatchedFilesParams = serde_json::from_value(notification.params).into_diagnostic()?;
                forget_languages();
                let changed: Vec<PathBuf> = params.changes.iter()
                    .filter_map(|change| path(&change.uri))
                    .flat_map(|file| [real(&file), file])
                    .collect();
                let uris: Vec<Uri> = self.documents.keys()
                    .filter(|uri| self.folders(uri).iter().any(|folder| changed.iter().any(|file| file.starts_with(folder))))
                    .cloned().collect();

                for uri in uris {
                    self.update(&uri, false);
                    self.trace(&uri);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn update(&self, uri: &Uri, changed: bool) {
        let Some((text, path)) = self.documents.get(uri).zip(path(uri)) else { return };

        self.shared.update(uri, diagnostics(&self.registry, &path, text), changed);
        if let Some(snapshot) = self.listing(uri) {
            self.worker.check(snapshot);
        }
    }

    fn trace(&self, uri: &Uri) {
        if let Some(snapshot) = self.snapshot(uri) {
            self.worker.trace(snapshot);
        }
    }

    fn remember_names(&mut self, uri: &Uri) -> bool {
        let Some((_, text)) = self.patch(uri) else { return false };
        let names = names("", text);

        self.names.insert(uri.clone(), names.clone()).is_some_and(|known| known != names)
    }

    fn toggle(&self, uri: &Uri, name: Option<&str>) -> Option<Edit> {
        let (_, text) = self.patch(uri)?;
        let views = views(text);
        let view = match name {
            Some(_) => views.iter().find(|view| view.name() == name),
            None => views.first().filter(|_| views.len() == 1)
        };

        toggled(uri, view?)
    }

    fn project(&self, uri: &Uri) -> Option<PathBuf> {
        canonicalize(options::project(&path(uri)?, self.documents.get(uri)?)?).ok()
    }

    fn folders(&self, uri: &Uri) -> Vec<PathBuf> {
        let Some((path, text)) = path(uri).zip(self.documents.get(uri)) else { return vec![] };

        let mut folders: Vec<PathBuf> = match kind(&path) {
            DocumentKind::Patch { project } => project.parent().map(PathBuf::from).into_iter().collect(),
            DocumentKind::Settings => path.parent().and_then(Path::parent).map(PathBuf::from).into_iter().collect(),
            DocumentKind::Configuration => path.parent().map(PathBuf::from).into_iter().collect(),
            DocumentKind::Other => vec![]
        };
        folders.extend(options::targets(&path, text, Default::default()).into_iter().map(|target| target.directory));
        folders.extend(folders.clone().into_iter().filter_map(|folder| canonicalize(folder).ok()));

        folders
    }

    fn target_files(&self, uri: &Uri, outline: &Outline) -> Vec<Location> {
        let Some((project, _)) = self.patch(uri) else { return vec![] };
        let Some((directory, files)) = target::files(&self.registry, &project, outline) else { return vec![] };

        files.iter()
            .filter_map(|file| Some(Location::new(file_uri(&directory.join(file))?, Range::default())))
            .collect()
    }
}


fn real(file: &Path) -> PathBuf {
    let found = file.ancestors().find_map(|ancestor| Some((ancestor, canonicalize(ancestor).ok()?)));

    match found {
        Some((ancestor, real)) if ancestor != file => real.join(file.strip_prefix(ancestor).unwrap_or(Path::new(""))),
        Some((_, real)) => real,
        None => file.to_path_buf()
    }
}

fn toggled(uri: &Uri, view: &View) -> Option<Edit> {
    let index = LineIndex::new(&view.text);
    let toggle = toggle(&Outline::parse(&view.text))?;
    let edit = TextEdit::new(index.range(toggle.value), toggle.text.to_string());

    Some((index.range(toggle.node), toggle.title, WorkspaceEdit::new(HashMap::from([(uri.clone(), vec![edit])]))))
}

fn remove_alias(uri: &Uri, project: &Path, text: &str, view: &View) -> Option<Edit> {
    let alias = redundant_alias(project, &view.text)?;
    let index = LineIndex::new(text);
    let start = text[..alias.start].rfind('\n').map(|line| line + 1).unwrap_or(0);
    let end = text[alias.end..].find('\n').map(|line| alias.end + line + 1).unwrap_or(text.len());
    let whole = text[start..alias.start].trim().is_empty() && text[alias.end..end].trim().is_empty();
    let separator = text[alias.end..].len() - text[alias.end..].trim_start_matches([' ', '\t']).trim_start_matches(';').len();
    let removed = if whole { start..end } else { alias.start..alias.end + separator };
    let edit = TextEdit::new(index.range(removed), String::new());

    Some((index.range(alias), "Remove alias", WorkspaceEdit::new(HashMap::from([(uri.clone(), vec![edit])]))))
}
