// Standard Uses
use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

// Crate Uses
use crate::diagnostics::{coverage, file};
use crate::hover::{hover, settings};
use crate::inlay::{places, Counts, Hints};
use crate::position::LineIndex;
use crate::preview::{configuration, operation_preview, preview};
use crate::syntax::Outline;
use crate::target;
use crate::trace::{Limits, Traces};

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::session::Preview;
use codeshaper::core::shaping::patch::trace::Trace;
use lsp_server::{ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::request::{CodeLensRefresh, InlayHintRefreshRequest, Request as _};
use lsp_types::{Diagnostic, Hover, HoverContents, Location, MarkupContent, MarkupKind, PublishDiagnosticsParams, Uri};
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
    Languages(usize),
    Preview,
    Operation { configuration: Option<PathBuf>, open: HashMap<PathBuf, String> }
}

enum Task {
    Trace(Snapshot),
    Request(RequestId, Snapshot, Work)
}

#[derive(Default)]
struct State {
    revision: u64,
    diagnostics: Vec<Diagnostic>,
    coverage: Vec<Diagnostic>,
    trace: Option<Arc<Trace>>,
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
            state.trace = None;
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

    pub fn trace(&self, uri: &Uri) -> Option<Arc<Trace>> {
        lock(&self.states).get(uri)?.trace.clone()
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

    fn traced(&self, snapshot: &Snapshot, coverage: Vec<Diagnostic>, trace: Option<Arc<Trace>>, counts: Counts) {
        let mut states = lock(&self.states);
        let Some(state) = states.get_mut(&snapshot.uri).filter(|state| state.revision == snapshot.revision) else { return };

        state.trace = trace;
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

    for task in tasks {
        match task {
            Task::Trace(snapshot) => {
                if !shared.is_current(&snapshot) { continue }

                let _ = catch_unwind(AssertUnwindSafe(|| trace(shared, registry, &mut traces, &snapshot)));
            }
            Task::Request(id, snapshot, work) => {
                if shared.is_cancelled(&id) {
                    shared.finish(id.clone(), Response::new_err(id, ErrorCode::RequestCanceled as i32, "Cancelled".to_string()));
                    continue
                }

                let response = catch_unwind(AssertUnwindSafe(|| answer(shared, registry, &mut traces, &id, &snapshot, work)))
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

    let mut found = file(registry, project, text);
    let trace = traces.get(registry, project, text);
    found.extend(trace.iter().flat_map(|trace| coverage(text, trace)));

    let mut counts = Counts::default();
    if shared.hints.matches {
        let outline = Outline::parse(text);

        for (place, _) in places(&outline) {
            if !shared.is_current(snapshot) { return }
            let Some(matches) = target::matches(registry, project, &outline, &place.location) else { continue };
            counts.insert(place, matches);
        }
    }

    shared.traced(snapshot, found, trace, counts);
}

fn answer(
    shared: &Arc<Shared>, registry: &Registry, traces: &mut Traces, id: &RequestId, snapshot: &Snapshot, work: Work
) -> Response {
    let Snapshot { uri, project, text, .. } = snapshot;
    let (guard, request, current) = (shared.clone(), id.clone(), snapshot.clone());
    traces.guard(move || !guard.is_cancelled(&request) && guard.is_current(&current));

    match work {
        Work::Hover(offset) => {
            hover_response(id, text, hover(registry, project, traces, text, offset))
        }
        Work::Languages(offset) => hover_response(id, text, settings(registry, project, text, offset)),
        Work::Places(location) => Response::new_ok(id.clone(), target_places(registry, project, text, &location)),
        Work::Preview => preview_response(registry, id, uri, project, text),
        Work::Operation { configuration: path, open } => {
            let keep_going = || !shared.is_cancelled(id);
            let found = path.map_or_else(|| configuration(project), Ok)
                .and_then(|path| operation_preview(registry, &path, &open, &keep_going));

            match found {
                Ok(preview) => operation_response(id, preview),
                Err(error) => Response::new_err(id.clone(), ErrorCode::RequestFailed as i32, message(&error))
            }
        }
    }
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

fn hover_response(id: &RequestId, text: &str, found: Option<(String, std::ops::Range<usize>)>) -> Response {
    let found = found.map(|(value, range)| Hover {
        contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
        range: Some(LineIndex::new(text).range(range))
    });

    Response::new_ok(id.clone(), found)
}

fn preview_response(registry: &Registry, id: &RequestId, uri: &Uri, project: &Path, text: &str) -> Response {
    let name = path(uri).and_then(|path| Some(path.file_name()?.to_string_lossy().to_string())).unwrap_or_default();

    let (target, changes) = match preview(registry, project, &name, text) {
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

    Response::new_ok(id.clone(), json!({ "matched": matched, "files": files, "errors": errors }))
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
    let related: Vec<String> = error.related().into_iter().flatten().map(|related| related.to_string()).collect();

    if related.is_empty() { cause } else { format!("{cause}: {}", related.join("; ")) }
}
