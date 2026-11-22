// Standard Uses
use std::path::Path;

// Crate Uses
use crate::document::ast_set;
use crate::schema::{self, LOCATIONS};
use crate::syntax::{Context, Outline};

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::patch_backend;
use codeshaper::core::shaping::patch::Patch;
use lsp_types::{CompletionItem, CompletionItemKind};


pub fn completion(registry: &Registry, project: &Path, text: &str, offset: usize) -> Vec<CompletionItem> {
    let outline = Outline::parse(text);

    match outline.context(text, offset) {
        Context::Name { parent, .. } => {
            let parent = parent.map(|parent| outline.nodes[parent].name.as_str());

            schema::children(parent).iter()
                .map(|name| item(name, CompletionItemKind::PROPERTY, None))
                .collect()
        }
        Context::Value { node, before, .. } => {
            if before.rfind("#[").is_some_and(|open| !before[open..].contains(']')) {
                return outline.actions()
                    .map(|(node, name)| item(&name.value, CompletionItemKind::FUNCTION, Some(&node.name)))
                    .collect()
            }

            let location = LOCATIONS.contains(&outline.nodes[node].name.as_str());
            if !location || before.trim_start().starts_with('(') {
                return vec![]
            }

            let Some(backend) = backend(registry, project, &outline) else { return vec![] };

            backend.location_names(before.matches('.').count()).iter()
                .map(|name| item(name, CompletionItemKind::CLASS, Some(backend.name())))
                .collect()
        }
        Context::None => vec![]
    }
}

pub fn backend<'r>(registry: &'r Registry, project: &Path, outline: &Outline) -> Option<&'r dyn LanguageBackend> {
    let patch = Patch {
        alias: outline.argument("alias").unwrap_or_default().to_string(),
        file: outline.argument("file").unwrap_or_default().to_string(),
        ..Patch::default()
    };

    patch_backend(registry, &patch, &ast_set(project))
}

fn item(label: &str, kind: CompletionItemKind, detail: Option<&str>) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: detail.map(str::to_string),
        ..CompletionItem::default()
    }
}
