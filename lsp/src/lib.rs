// Relative Modules
pub mod diagnostics;
pub mod document;
pub mod position;
pub mod server;

// Standard Uses

// Crate Uses
use crate::server::Server;

// External Uses
use lsp_server::Connection;
use lsp_types::{ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind};
use miette::{miette, IntoDiagnostic, Result};


pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..ServerCapabilities::default()
    }
}

pub fn run() -> Result<()> {
    let (connection, threads) = Connection::stdio();

    serve(&connection)?;
    drop(connection);

    threads.join().into_diagnostic()
}

pub fn serve(connection: &Connection) -> Result<()> {
    let capabilities = serde_json::to_value(capabilities()).into_diagnostic()?;
    connection.initialize(capabilities).map_err(|error| miette!("{error}"))?;

    Server::new(connection, codeshaper::registry()).serve()
}
