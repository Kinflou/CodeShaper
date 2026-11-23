// Standard Uses
use std::ops::Range;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

// Crate Uses
use crate::diagnostics::UNCOVERED;
use crate::preview::operation;
use crate::protocol::Client;

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use lsp_server::{Message, Request, RequestId, Response};
use lsp_types::{InitializeParams, Uri};
use miette::Result;
use serde_json::{json, Value};


#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    opened: Condvar,
    seen: Mutex<Vec<String>>,
    broken: bool
}

impl Gate {
    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.opened.notify_all();
    }

    fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }

    fn wait_until_reached(&self) {
        while self.seen().is_empty() { thread::sleep(Duration::from_millis(5)); }
    }
}

struct SlowBackend(Arc<Gate>);

impl LanguageBackend for SlowBackend {
    fn name(&self) -> &str { "cpp" }

    fn locate(&self, source: &str, _location: &str) -> Result<Vec<Range<usize>>> {
        self.0.seen.lock().unwrap().push(source.to_string());
        assert!(!self.0.broken, "broken backend");

        let mut open = self.0.open.lock().unwrap();
        while !*open { open = self.0.opened.wait(open).unwrap(); }

        Ok(vec![])
    }
}

fn start(gate: &Arc<Gate>) -> Client {
    let gate = gate.clone();

    Client::start_on(InitializeParams::default(), move || {
        let mut registry = Registry::default();
        registry.add_backend(Box::new(SlowBackend(gate.clone())));
        codeshaper_target_text::register(&mut registry);
        registry
    })
}

fn ask(client: &Client, id: i32, method: &str, uri: &Uri, line: u32) {
    let params = json!({ "textDocument": { "uri": uri.as_str() }, "position": { "line": line, "character": 25 } });

    client.connection.sender.send(Request::new(RequestId::from(id), method.to_string(), params).into()).unwrap();
}

fn until(client: &Client, id: i32) -> (Response, Vec<Value>) {
    let mut published = vec![];

    loop {
        match client.receive() {
            Message::Response(response) if response.id == RequestId::from(id) => return (response, published),
            Message::Notification(notification) => published.push(notification.params),
            _ => {}
        }
    }
}


#[test]
fn completion_answers_while_a_trace_runs() {
    let (_directory, project) = operation();
    let gate = Arc::new(Gate::default());
    let client = start(&gate);
    let uri = client.open(&project.join("patches/uncovered.kdl"), UNCOVERED);
    gate.wait_until_reached();

    ask(&client, 50, "textDocument/completion", &uri, 3);
    assert!(until(&client, 50).0.response_result.is_ok());
    assert!(!*gate.open.lock().unwrap());

    gate.open();
    client.stop();
}

#[test]
fn edits_stop_the_running_trace() {
    let (_directory, project) = operation();
    let gate = Arc::new(Gate::default());
    let client = start(&gate);
    let uri = client.open(&project.join("patches/uncovered.kdl"), UNCOVERED);
    gate.wait_until_reached();

    client.notify("textDocument/didChange", json!({
        "textDocument": { "uri": uri.as_str(), "version": 2 },
        "contentChanges": [{ "text": "enabled true\n" }]
    }));
    ask(&client, 50, "textDocument/foldingRange", &uri, 0);
    until(&client, 50);

    gate.open();
    ask(&client, 51, "textDocument/hover", &uri, 0);
    let (_, published) = until(&client, 51);

    assert_eq!(published, Vec::<Value>::new());
    assert!(gate.seen().iter().all(|source| source.contains("void old")), "{:?}", gate.seen());

    client.stop();
}

#[test]
fn queued_requests_are_cancelled() {
    let (_directory, project) = operation();
    let gate = Arc::new(Gate::default());
    let client = start(&gate);
    let uri = client.open(&project.join("patches/uncovered.kdl"), UNCOVERED);
    gate.wait_until_reached();

    ask(&client, 50, "textDocument/hover", &uri, 18);
    client.notify("$/cancelRequest", json!({ "id": 50 }));
    ask(&client, 51, "textDocument/foldingRange", &uri, 0);
    until(&client, 51);

    gate.open();
    let (response, _) = until(&client, 50);
    assert_eq!(response.response_result.unwrap_err().code, -32800);

    client.stop();
}

#[test]
fn panics_become_errors() {
    let (_directory, project) = operation();
    let gate = Arc::new(Gate { broken: true, ..Gate::default() });
    let client = start(&gate);
    let uri = client.open(&project.join("patches/uncovered.kdl"), UNCOVERED);

    for id in [50, 51] {
        ask(&client, id, "textDocument/hover", &uri, 18);
        assert_eq!(until(&client, id).0.response_result.unwrap_err().code, -32603);
    }

    client.stop();
}

#[test]
fn shutdown_stops_the_running_trace() {
    let (_directory, project) = operation();
    let gate = Arc::new(Gate::default());
    let client = start(&gate);
    client.open(&project.join("patches/uncovered.kdl"), UNCOVERED);
    gate.wait_until_reached();

    let later = gate.clone();
    let opener = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        later.open();
    });

    client.stop();
    opener.join().unwrap();
    assert!(gate.seen().iter().all(|source| source.contains("void old")), "{:?}", gate.seen());
}
