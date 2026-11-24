// Relative Modules
pub mod completion;
pub mod diagnostics;
pub mod document;
pub mod groups;
pub mod hover;
pub mod inlay;
pub mod navigation;
pub mod options;
pub mod paths;
pub mod position;
pub mod preview;
pub mod query;
pub mod schema;
pub mod server;
pub mod symbols;
pub mod syntax;
pub mod target;
pub mod toggle;
pub mod tokens;
pub mod trace;
pub mod worker;

// Standard Uses

// Crate Uses
use crate::inlay::Hints;
use crate::server::{Options, Server, TOGGLE_COMMAND};
use crate::tokens::LEGEND;
use crate::trace::Limits;
use crate::worker::Refresh;

// External Uses
use codeshaper::core::registry::Registry;
use lsp_server::Connection;
use lsp_types::{
    CodeActionProviderCapability, CodeLensOptions, CompletionOptions, ExecuteCommandOptions, FoldingRangeProviderCapability, HoverProviderCapability, OneOf, RenameOptions, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind
};
use miette::{miette, IntoDiagnostic, Result};


pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(["[", ".", "\"", "(", "#", "@", "/"].map(str::to_string).to_vec()),
            ..CompletionOptions::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        code_lens_provider: Some(CodeLensOptions { resolve_provider: Some(false) }),
        definition_provider: Some(OneOf::Left(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        execute_command_provider: Some(ExecuteCommandOptions { commands: vec![TOGGLE_COMMAND.to_string()], ..ExecuteCommandOptions::default() }),
        rename_provider: Some(OneOf::Right(RenameOptions { prepare_provider: Some(true), work_done_progress_options: Default::default() })),
        document_symbol_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
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
    serve_with(connection, codeshaper::registry)
}

pub fn serve_with(connection: &Connection, registry: impl Fn() -> Registry + Send + 'static) -> Result<()> {
    let capabilities = serde_json::to_value(capabilities()).into_diagnostic()?;
    let parameters = connection.initialize(capabilities).map_err(|error| miette!("{error}"))?;

    let options = Options {
        limits: limits(&parameters),
        refresh: Refresh {
            lenses: parameters["capabilities"]["workspace"]["codeLens"]["refreshSupport"].as_bool().unwrap_or(false),
            hints: parameters["capabilities"]["workspace"]["inlayHint"]["refreshSupport"].as_bool().unwrap_or(false)
        },
        hints: Hints {
            groups: parameters["initializationOptions"]["inlayHints"]["groups"].as_bool().unwrap_or(false),
            matches: parameters["initializationOptions"]["inlayHints"]["matches"].as_bool().unwrap_or(false)
        },
        definition_links: parameters["capabilities"]["textDocument"]["definition"]["linkSupport"].as_bool().unwrap_or(false),
        suggest_again: parameters["initializationOptions"]["suggestAgain"].as_bool().unwrap_or(false),
        watch: parameters["capabilities"]["workspace"]["didChangeWatchedFiles"]["dynamicRegistration"].as_bool().unwrap_or(false)
    };

    Server::new(connection, registry, options).serve()
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
