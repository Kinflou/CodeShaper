// Relative Modules
pub mod completion;
pub mod diagnostics;
pub mod document;
pub mod hover;
pub mod navigation;
pub mod position;
pub mod preview;
pub mod schema;
pub mod server;
pub mod syntax;
pub mod tokens;
pub mod trace;

// Standard Uses

// Crate Uses
use crate::server::{Options, Server};
use crate::tokens::LEGEND;
use crate::trace::Limits;

// External Uses
use lsp_server::Connection;
use lsp_types::{
    CodeLensOptions, CompletionOptions, HoverProviderCapability, OneOf, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
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
        code_lens_provider: Some(CodeLensOptions { resolve_provider: Some(false) }),
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
    let parameters = connection.initialize(capabilities).map_err(|error| miette!("{error}"))?;

    let options = Options {
        limits: limits(&parameters),
        refresh_lenses: parameters["capabilities"]["workspace"]["codeLens"]["refreshSupport"].as_bool().unwrap_or(false)
    };

    Server::new(connection, codeshaper::registry(), options).serve()
}

fn limits(parameters: &serde_json::Value) -> Limits {
    let hover = &parameters["initializationOptions"]["hover"];
    let read = |name: &str, default: usize| hover[name].as_u64().map(|value| value as usize).unwrap_or(default);
    let defaults = Limits::default();

    Limits {
        samples: read("samples", defaults.samples),
        lines: read("lines", defaults.lines),
        characters: read("characters", defaults.characters)
    }
}
