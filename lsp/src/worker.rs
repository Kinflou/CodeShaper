// Standard Uses
use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

// Crate Uses
use crate::diagnostics::{coverage, duplicates, file};
use crate::document::{kind, DocumentKind};
use crate::explore::{self, Located};
use crate::hover::{hover, settings};
use crate::inlay::{places, Counts, Hints, Place};
use crate::navigation::links;
use crate::paths::{self, check_patterns};
use crate::position::LineIndex;
use crate::preview::{configuration, operation_preview, preview};
use crate::section::views;
use crate::syntax::Outline;
use crate::target;
use crate::trace::{Limits, Traces};

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::session::Preview;
use codeshaper::core::shaping::patch::trace::Trace;
use codeshaper_target_text::options::Field;
use lsp_server::{ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::request::{CodeLensRefresh, InlayHintRefreshRequest, Request as _};
use lsp_types::{Diagnostic, Hover, HoverContents, Location, MarkupContent, MarkupKind, PublishDiagnosticsParams, Range, Uri};
use miette::{SourceCode, SourceSpan};
use serde_json::json;
use url::Url;


#[derive(Debug, Clone)]
pub struct Snapshot {
    pub uri: Uri,
    pub project: PathBuf,
    pub text: String,
    pub revision: u64
}

#[derive(Debug, Clone)]
pub enum Work {
    Hover(usize),
    Places(String),
    Explain(usize),
    Open { field: Field, pattern: String, origin: Range, links: bool },
    Preview(Option<String>),
    Operation { configuration: Option<PathBuf>, open: HashMap<PathBuf, String> },
    Explore { method: String, start: PathBuf, params: serde_json::Value, open: HashMap<PathBuf, String> }
}

enum Task {
    Trace(Snapshot),
    Check(Snapshot),
    Request(RequestId, Snapshot, Work)
}

#[derive(Default)]
struct State {
    revision: u64,
    diagnostics: Vec<Diagnostic>,
    coverage: Vec<Diagnostic>,
    traces: Vec<Option<Arc<Trace>>>,
    counts: Counts
}

#[derive(Default)]
struct Requests {
    pending: HashSet<RequestId>,
    cancelled: HashSet<RequestId>
}

pub struct Shared {
    sender: Outgoing,
    states: Mutex<HashMap<Uri, State>>,
    requests: Mutex<Requests>,
    stopping: AtomicBool,
    revisions: AtomicU64,
    asked: AtomicI32,
    refresh: Refresh,
    hints: Hints
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Refresh {
    pub lenses: bool,
    pub hints: bool
}

type Outgoing = Box<dyn Fn(Message) + Send + Sync>;


impl Shared {
    pub fn new(sender: impl Fn(Message) + Send + Sync + 'static, refresh: Refresh, hints: Hints) -> Self {
        Self {
            sender: Box::new(sender),
            states: Mutex::default(),
            requests: Mutex::default(),
            stopping: AtomicBool::default(),
            revisions: AtomicU64::default(),
            asked: AtomicI32::default(),
            refresh,
            hints
        }
    }

    pub fn send(&self, message: Message) {
        (self.sender)(message)
    }

    pub fn ask(&self, method: &str, params: serde_json::Value) {
        let id = self.asked.fetch_add(1, Ordering::Relaxed) + 1;

        self.send(Request::new(RequestId::from(format!("server-{id}")), method.to_string(), params).into())
    }

    pub fn update(&self, uri: &Uri, diagnostics: Vec<Diagnostic>, changed: bool) {
        let revision = self.revisions.fetch_add(1, Ordering::Relaxed) + 1;
        let mut states = lock(&self.states);
        let state = states.entry(uri.clone()).or_default();

        state.revision = revision;
        state.diagnostics = diagnostics;
        if changed {
            state.coverage.clear();
            state.traces.clear();
        }
        self.publish(uri, state);
    }

    pub fn close(&self, uri: &Uri) {
        let mut states = lock(&self.states);

        states.remove(uri);
        self.publish(uri, &State::default());
    }

    pub fn revision(&self, uri: &Uri) -> Option<u64> {
        lock(&self.states).get(uri).map(|state| state.revision)
    }

    pub fn traces(&self, uri: &Uri) -> Vec<Option<Arc<Trace>>> {
        lock(&self.states).get(uri).map(|state| state.traces.clone()).unwrap_or_default()
    }

    pub fn counts(&self, uri: &Uri) -> Counts {
        lock(&self.states).get(uri).map(|state| state.counts.clone()).unwrap_or_default()
    }

    pub fn cancel(&self, id: RequestId) {
        let mut requests = lock(&self.requests);

        if requests.pending.contains(&id) {
            requests.cancelled.insert(id);
        }
    }

    fn traced(&self, snapshot: &Snapshot, coverage: Vec<Diagnostic>, traces: Vec<Option<Arc<Trace>>>, counts: Counts) {
        let mut states = lock(&self.states);
        let Some(state) = states.get_mut(&snapshot.uri).filter(|state| state.revision == snapshot.revision) else { return };

        state.traces = traces;
        state.counts = counts;
        if state.coverage != coverage {
            state.coverage = coverage;
            self.publish(&snapshot.uri, state);
        }
        drop(states);

        if self.refresh.lenses {
            self.ask(CodeLensRefresh::METHOD, json!(null));
        }
        if self.refresh.hints && self.hints.matches {
            self.ask(InlayHintRefreshRequest::METHOD, json!(null));
        }
    }

    fn checked(&self, snapshot: &Snapshot, found: Vec<Diagnostic>) {
        let mut states = lock(&self.states);
        let Some(state) = states.get_mut(&snapshot.uri).filter(|state| state.revision == snapshot.revision) else { return };

        if state.coverage != found {
            state.coverage = found;
            self.publish(&snapshot.uri, state);
        }
    }

    fn is_current(&self, snapshot: &Snapshot) -> bool {
        !self.stopping.load(Ordering::Relaxed)
            && lock(&self.states).get(&snapshot.uri).is_some_and(|state| state.revision == snapshot.revision)
    }

    fn is_cancelled(&self, id: &RequestId) -> bool {
        self.stopping.load(Ordering::Relaxed) || lock(&self.requests).cancelled.contains(id)
    }

    fn finish(&self, id: RequestId, response: Response) {
        let mut requests = lock(&self.requests);
        let cancelled = requests.cancelled.remove(&id);
        requests.pending.remove(&id);
        drop(requests);

        self.send(match cancelled {
            true => Response::new_err(id, ErrorCode::RequestCanceled as i32, "Cancelled".to_string()).into(),
            false => response.into()
        })
    }

    fn publish(&self, uri: &Uri, state: &State) {
        let diagnostics = state.diagnostics.iter().chain(&state.coverage).cloned().collect();
        let params = PublishDiagnosticsParams { uri: uri.clone(), diagnostics, version: None };

        self.send(Notification::new(PublishDiagnostics::METHOD.to_string(), params).into())
    }
}


pub struct Worker {
    shared: Arc<Shared>,
    tasks: Option<Sender<Task>>,
    thread: Option<JoinHandle<()>>
}

impl Worker {
    pub fn start(shared: Arc<Shared>, registry: impl FnOnce() -> Registry + Send + 'static, limits: Limits) -> Self {
        let (tasks, received) = channel();
        let state = shared.clone();
        let thread = thread::spawn(move || run(&state, &registry(), limits, received));

        Self { shared, tasks: Some(tasks), thread: Some(thread) }
    }

    pub fn trace(&self, snapshot: Snapshot) {
        self.queue(Task::Trace(snapshot));
    }

    pub fn check(&self, snapshot: Snapshot) {
        self.queue(Task::Check(snapshot));
    }

    pub fn request(&self, id: RequestId, snapshot: Snapshot, work: Work) {
        lock(&self.shared.requests).pending.insert(id.clone());
        self.queue(Task::Request(id, snapshot, work));
    }

    fn queue(&self, task: Task) {
        if let Some(tasks) = &self.tasks {
            let _ = tasks.send(task);
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.shared.stopping.store(true, Ordering::Relaxed);
        self.tasks = None;

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}


fn run(shared: &Arc<Shared>, registry: &Registry, limits: Limits, tasks: Receiver<Task>) {
    let mut traces = Traces::new(limits);
    let mut located = Located::default();

    for task in tasks {
        match task {
            Task::Trace(snapshot) => {
                if !shared.is_current(&snapshot) { continue }

                let _ = catch_unwind(AssertUnwindSafe(|| trace(shared, registry, &mut traces, &snapshot)));
            }
            Task::Check(snapshot) => {
                if !shared.is_current(&snapshot) { continue }

                let _ = catch_unwind(AssertUnwindSafe(|| {
                    let found = path(&snapshot.uri)
                        .and_then(|path| check_patterns(&path, &snapshot.text, &|| shared.is_current(&snapshot)));
                    if let Some(found) = found { shared.checked(&snapshot, found) }
                }));
            }
            Task::Request(id, snapshot, work) => {
                if shared.is_cancelled(&id) {
                    shared.finish(id.clone(), Response::new_err(id, ErrorCode::RequestCanceled as i32, "Cancelled".to_string()));
                    continue
                }

                let response = catch_unwind(AssertUnwindSafe(|| answer(shared, registry, &mut traces, &mut located, &id, &snapshot, work)))
                    .unwrap_or_else(|_| Response::new_err(id.clone(), ErrorCode::InternalError as i32, "The request failed unexpectedly".to_string()));
                shared.finish(id, response);
            }
        }
    }
}

fn trace(shared: &Arc<Shared>, registry: &Registry, traces: &mut Traces, snapshot: &Snapshot) {
    let Snapshot { project, text, .. } = snapshot;
    let (guard, current) = (shared.clone(), snapshot.clone());
    traces.guard(move || guard.is_current(&current));

    let mut found = path(&snapshot.uri).map(|path| duplicates(project, &path, text)).unwrap_or_default();
    let mut traced = vec![];
    let mut counts = Counts::default();

    for view in views(text) {
        found.extend(file(registry, project, &view.text));
        let trace = traces.get(registry, project, &view.text);
        found.extend(trace.iter().flat_map(|trace| coverage(&view.text, trace)));
        traced.push(trace);

        if !shared.hints.matches { continue }
        let outline = Outline::parse(&view.text);

        let mut places: Vec<Place> = places(&outline).into_iter().map(|(place, _)| place).collect();
        places.sort_by(|one, other| one.location.cmp(&other.location));
        places.dedup();

        let locations: Vec<&str> = places.iter().map(|place| place.location.as_str()).collect();
        let found = traces.places(registry, project, &target::patch(&outline), &locations);
        if !shared.is_current(snapshot) { return }

        counts.extend(places.into_iter().zip(found.into_iter().flat_map(|(_, found)| found).map(|located| located.matches)));
    }

    shared.traced(snapshot, found, traced, counts);
}

fn answer(
    shared: &Arc<Shared>, registry: &Registry, traces: &mut Traces, located: &mut Located, id: &RequestId, snapshot: &Snapshot, work: Work
) -> Response {
    let Snapshot { uri, project, text, .. } = snapshot;
    let (guard, request, current) = (shared.clone(), id.clone(), snapshot.clone());
    traces.guard(move || !guard.is_cancelled(&request) && guard.is_current(&current));

    match work {
        Work::Hover(offset) => {
            hover_response(id, text, hover(registry, project, traces, text, offset))
        }
        Work::Explain(offset) => {
            let found = path(uri).and_then(|path| {
                let listed = paths::hover(&path, text, offset, &|| !shared.is_cancelled(id));

                listed.or_else(|| settings(registry, project, text, offset).filter(|_| kind(&path) == DocumentKind::Settings))
            });

            hover_response(id, text, found)
        }
        Work::Open { field, pattern, origin, links: wanted } => {
            let found: Vec<Location> = path(uri)
                .map(|path| paths::open(&path, text, field, &pattern, &|| !shared.is_cancelled(id)))
                .unwrap_or_default().iter()
                .filter_map(|file| Some(Location::new(file_uri(file)?, Range::default())))
                .collect();

            Response::new_ok(id.clone(), links(wanted, origin, found))
        }
        Work::Places(location) => Response::new_ok(id.clone(), target_places(registry, project, text, &location)),
        Work::Preview(name) => preview_response(registry, id, uri, project, name, text),
        Work::Operation { configuration: path, open } => {
            let keep_going = || !shared.is_cancelled(id);
            let found = path.map_or_else(|| configuration(project), Ok)
                .and_then(|path| operation_preview(registry, &path, &open, &keep_going));

            match found {
                Ok(preview) => operation_response(id, preview),
                Err(error) => Response::new_err(id.clone(), ErrorCode::RequestFailed as i32, message(&error))
            }
        }
        Work::Explore { method, start, params, open } => {
            match explore::answer(registry, located, &method, &start, &params, &open, &|| !shared.is_cancelled(id)) {
                Ok(found) => Response::new_ok(id.clone(), found),
                Err(error) => Response::new_err(id.clone(), ErrorCode::RequestFailed as i32, message(&error))
            }
        }
    }
}

fn hover_response(id: &RequestId, text: &str, found: Option<(String, std::ops::Range<usize>)>) -> Response {
    let found = found.map(|(value, range)| Hover {
        contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
        range: Some(LineIndex::new(text).range(range))
    });

    Response::new_ok(id.clone(), found)
}

fn target_places(registry: &Registry, project: &Path, text: &str, location: &str) -> Vec<Location> {
    let mut found = vec![];

    for places in target::places(registry, project, &Outline::parse(text), location) {
        let Some(uri) = file_uri(&places.path) else { continue };
        let index = LineIndex::new(&places.source);

        found.extend(places.ranges.into_iter().map(|range| Location::new(uri.clone(), index.range(range))));
    }

    found
}

fn preview_response(registry: &Registry, id: &RequestId, uri: &Uri, project: &Path, name: Option<String>, text: &str) -> Response {
    let file = || path(uri).and_then(|path| Some(path.file_name()?.to_string_lossy().to_string())).unwrap_or_default();

    let (target, changes) = match preview(registry, project, &name.clone().unwrap_or_else(file), text) {
        Ok(previewed) => previewed,
        Err(error) => return Response::new_err(id.clone(), ErrorCode::RequestFailed as i32, message(&error))
    };

    let matched = changes.len();
    let mut files = vec![];
    let mut errors = vec![];

    for change in changes {
        let path = change.file.display().to_string();

        match change.result {
            Ok(result) if result != change.before => files.push(json!({
                "uri": file_uri(&target.join(&change.file)).map(|uri| uri.to_string()).unwrap_or_default(),
                "path": path,
                "before": change.before,
                "result": result
            })),
            Ok(_) => {}
            Err(error) => errors.push(json!({ "path": path, "message": message(&error) }))
        }
    }

    Response::new_ok(id.clone(), json!({ "patch": name, "matched": matched, "files": files, "errors": errors }))
}

fn operation_response(id: &RequestId, preview: Preview) -> Response {
    let files: Vec<serde_json::Value> = preview.files.into_iter()
        .map(|file| json!({
            "uri": file_uri(&preview.directory.join(&file.path)).map(|uri| uri.to_string()).unwrap_or_default(),
            "path": file.path.display().to_string(),
            "before": file.before,
            "result": file.result
        }))
        .collect();
    let errors: Vec<serde_json::Value> = preview.errors.iter()
        .map(|(path, error)| json!({ "path": path.display().to_string(), "message": message(error) }))
        .collect();

    Response::new_ok(id.clone(), json!({ "files": files, "errors": errors }))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn path(uri: &Uri) -> Option<PathBuf> {
    Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}

pub fn file_uri(path: &Path) -> Option<Uri> {
    Url::from_file_path(path).ok()?.as_str().parse().ok()
}

fn message(error: &miette::Report) -> String {
    let cause = error.chain().map(|cause| cause.to_string()).collect::<Vec<_>>().join(": ");
    let related: Vec<String> = error.related().into_iter().flatten()
        .map(|related| match place(related, related.source_code().or(error.source_code())) {
            Some(place) => format!("{place}: {related}"),
            None => related.to_string()
        })
        .collect();

    if related.is_empty() { cause } else { format!("{cause}: {}", related.join("; ")) }
}

fn place(error: &dyn miette::Diagnostic, source: Option<&dyn SourceCode>) -> Option<String> {
    let label = error.labels().and_then(|mut labels| labels.next());
    let end = label.as_ref().map(|label| label.offset()).unwrap_or_default();
    let contents = source?.read_span(&SourceSpan::from(0..end), 0, 0).ok()?;
    let name = contents.name()?;

    if label.is_none() {
        return Some(name.to_string())
    }

    let before = String::from_utf8_lossy(contents.data().get(..end)?);
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or_default().chars().count() + 1;

    Some(format!("{name}:{line}:{column}"))
}
