// Standard Uses
use std::thread;
use std::time::Duration;

// Crate Uses
use crate::{project, BROKEN};

// External Uses
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::notification::{DidOpenTextDocument, Exit, Initialized, Notification as _, PublishDiagnostics};
use lsp_types::request::{Initialize, Request as _, Shutdown};
use lsp_types::{
    DidOpenTextDocumentParams, InitializeParams, InitializedParams, PublishDiagnosticsParams, TextDocumentItem, Uri
};
use url::Url;


fn receive(client: &Connection) -> Message {
    client.receiver.recv_timeout(Duration::from_secs(10)).unwrap()
}


#[test]
fn diagnostics_are_published_on_open() {
    let (_directory, project) = project(&[("broken.kdl", BROKEN)]);
    let path = project.join("patches/broken.kdl");
    let uri: Uri = Url::from_file_path(&path).unwrap().as_str().parse().unwrap();

    let (server, client) = Connection::memory();
    let handle = thread::spawn(move || codeshaper_lsp::serve(&server).map_err(|error| error.to_string()));

    client.sender.send(Request::new(RequestId::from(1), Initialize::METHOD.to_string(), InitializeParams::default()).into()).unwrap();
    assert!(matches!(receive(&client), Message::Response(response) if response.id == RequestId::from(1)));
    client.sender.send(Notification::new(Initialized::METHOD.to_string(), InitializedParams {}).into()).unwrap();

    let document = TextDocumentItem::new(uri.clone(), "kdl".to_string(), 1, BROKEN.to_string());
    client.sender.send(Notification::new(
        DidOpenTextDocument::METHOD.to_string(), DidOpenTextDocumentParams { text_document: document }
    ).into()).unwrap();

    let Message::Notification(published) = receive(&client) else { panic!("expected a notification") };
    assert_eq!(published.method, PublishDiagnostics::METHOD);
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert_eq!(params.uri, uri);
    assert_eq!(params.diagnostics.len(), 3);

    client.sender.send(Request::new(RequestId::from(2), Shutdown::METHOD.to_string(), ()).into()).unwrap();
    assert!(matches!(receive(&client), Message::Response(response) if response.id == RequestId::from(2)));
    client.sender.send(Notification::new(Exit::METHOD.to_string(), ()).into()).unwrap();

    assert_eq!(handle.join().unwrap(), Ok(()));
}
