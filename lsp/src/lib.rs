// Relative Modules
pub mod completion;
pub mod diagnostics;
pub mod document;
pub mod hover;
pub mod navigation;
pub mod position;
pub mod schema;
pub mod server;
pub mod syntax;
pub mod tokens;

// Standard Uses

// Crate Uses
use crate::server::Server;
use crate::tokens::LEGEND;

// External Uses
use lsp_server::Connection;
use lsp_types::{
    CompletionOptions, HoverProviderCapability, OneOf, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind
};
use miette::{miette, IntoDiagnostic, Result};


pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(["[", ".", "\""].map(str::to_string).to_vec()),
            ..CompletionOptions::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
            legend: SemanticTokensLegend { token_types: LEGEND.to_vec(), token_modifiers: vec![] },
            full: Some(SemanticTokensFullOptions::Bool(true)),
            ..SemanticTokensOptions::default()
        })),
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
