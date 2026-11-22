// Standard Uses
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Crate Uses
use crate::{project, BROKEN};

// External Uses
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidOpenTextDocument, Exit, Initialized, Notification as _, PublishDiagnostics};
use lsp_types::request::{Completion, Initialize, Request as _, SemanticTokensFullRequest, Shutdown};
use lsp_types::{
    CompletionParams, CompletionResponse, DidOpenTextDocumentParams, InitializeParams, InitializedParams, Position,
    PublishDiagnosticsParams, SemanticTokensParams, SemanticTokensResult, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Uri
};
use url::Url;


macro_rules! to_json {
    ($value:expr) => { serde_json::to_value($value).unwrap() };
}


pub struct Client {
    connection: Connection,
    server: Option<JoinHandle<Result<(), String>>>,
    next: i32
}

impl Client {
    pub fn start() -> Self {
        let (server, connection) = Connection::memory();
        let handle = thread::spawn(move || codeshaper_lsp::serve(&server).map_err(|error| error.to_string()));
        let mut client = Self { connection, server: Some(handle), next: 0 };

        client.request(Initialize::METHOD, to_json!(InitializeParams::default()));
        client.notify(Initialized::METHOD, to_json!(InitializedParams {}));
        client
    }

    pub fn open(&self, path: &Path, text: &str) -> Uri {
        let uri: Uri = Url::from_file_path(path).unwrap().as_str().parse().unwrap();
        let document = TextDocumentItem::new(uri.clone(), "kdl".to_string(), 1, text.to_string());

        self.notify(DidOpenTextDocument::METHOD, to_json!(DidOpenTextDocumentParams { text_document: document }));
        uri
    }

    pub fn request(&mut self, method: &str, params: serde_json::Value) -> Response {
        self.next += 1;
        let id = RequestId::from(self.next);
        self.connection.sender.send(Request::new(id.clone(), method.to_string(), params).into()).unwrap();

        loop {
            if let Message::Response(response) = self.receive() {
                if response.id == id { return response }
            }
        }
    }

    pub fn notify(&self, method: &str, params: serde_json::Value) {
        self.connection.sender.send(Notification::new(method.to_string(), params).into()).unwrap();
    }

    pub fn receive(&self) -> Message {
        self.connection.receiver.recv_timeout(Duration::from_secs(10)).unwrap()
    }

    pub fn stop(mut self) {
        self.request(Shutdown::METHOD, serde_json::Value::Null);
        self.notify(Exit::METHOD, serde_json::Value::Null);

        assert_eq!(self.server.take().unwrap().join().unwrap(), Ok(()));
    }
}

pub fn at(uri: &Uri, position: Position) -> TextDocumentPositionParams {
    TextDocumentPositionParams::new(TextDocumentIdentifier::new(uri.clone()), position)
}


#[test]
fn diagnostics_are_published_on_open() {
    let (_directory, project) = project(&[("broken.kdl", BROKEN)]);
    let client = Client::start();

    let uri = client.open(&project.join("patches/broken.kdl"), BROKEN);

    let Message::Notification(published) = client.receive() else { panic!("expected a notification") };
    assert_eq!(published.method, PublishDiagnostics::METHOD);
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert_eq!(params.uri, uri);
    assert_eq!(params.diagnostics.len(), 3);

    client.stop();
}

#[test]
fn completion_offers_fields() {
    let (_directory, project) = project(&[("partial.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/partial.kdl"), "actions {\n    makers {\n        maker \"m\" {\n            ma\n");

    let params = CompletionParams {
        text_document_position: at(&uri, Position::new(3, 14)),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None
    };
    let response = client.request(Completion::METHOD, to_json!(params));
    let Some(CompletionResponse::Array(items)) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected completion items")
    };

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), vec!["prepare", "make"]);

    client.stop();
}

#[test]
fn semantic_tokens_for_expressions() {
    let (_directory, project) = project(&[("tokens.kdl", "")]);
    let mut client = Client::start();
    let uri = client.open(&project.join("patches/tokens.kdl"), "to \"$[0] #[names]\"\n");

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier::new(uri),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default()
    };
    let response = client.request(SemanticTokensFullRequest::METHOD, to_json!(params));
    let Some(SemanticTokensResult::Tokens(tokens)) = serde_json::from_value(response.response_result.unwrap()).unwrap() else {
        panic!("expected semantic tokens")
    };

    assert_eq!(tokens.data.iter().map(|token| (token.delta_start, token.length, token.token_type)).collect::<Vec<_>>(), vec![
        (4, 4, 1), (5, 8, 0)
    ]);

    client.stop();
}
