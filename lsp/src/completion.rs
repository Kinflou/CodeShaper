// Standard Uses
use std::collections::{BTreeSet, HashSet};
use std::ops::Range;
use std::path::Path;

// Crate Uses
use crate::document::{kind, DocumentKind};
use crate::groups::groups;
use crate::options;
use crate::paths;
use crate::position::LineIndex;
use crate::query::{captures, is_query, part_at, Part, PREDICATES};
use crate::schema::{self, regex_field, LOCATIONS};
use crate::syntax::{Context, Outline};
use crate::target::{backends, every_file};

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::project::ast_set::TARGET;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, TextEdit};


pub fn completion(registry: &Registry, project: &Path, text: &str, offset: usize) -> Vec<CompletionItem> {
    let outline = Outline::parse(text);

    match outline.context(text, offset) {
        Context::Name { parent, .. } => {
            let parent = parent.map(|parent| outline.nodes[parent].name.as_str());

            schema::children(parent).iter()
                .map(|name| item(name, CompletionItemKind::PROPERTY, None))
                .collect()
        }
        Context::Value { node, literal, before } => {
            if outline.is_top(node, "file") {
                return paths::target(text, &literal.content, offset, &every_file(registry, project))
            }

            if outline.is_top(node, "alias") {
                return registry.languages().into_iter().map(|name| item(name, CompletionItemKind::VALUE, None)).collect()
            }

            if before.rfind("#[").is_some_and(|open| !before[open..].contains(']')) {
                return outline.actions()
                    .map(|(node, name)| item(&name.value, CompletionItemKind::FUNCTION, Some(&node.name)))
                    .collect()
            }

            if before.rfind("$[").is_some_and(|open| !before[open..].contains(']')) {
                let Some(action) = outline.nodes[node].parent else { return vec![] };
                let Some(pattern) = regex_field(&outline.nodes[action].name).and_then(|field| outline.field(action, field)) else {
                    return vec![]
                };

                return groups(&outline, action).into_iter().enumerate()
                    .flat_map(|(index, group)| group.name.into_iter().chain([index.to_string()])
                        .map(|label| item(&label, CompletionItemKind::VARIABLE, Some(&pattern.value[group.range.clone()])))
                        .collect::<Vec<_>>())
                    .collect()
            }

            let location = LOCATIONS.contains(&outline.nodes[node].name.as_str());
            if !location { return vec![] }

            let mut found: Vec<CompletionItem> = vec![];
            for backend in backends(registry, project, &outline) {
                let items = match is_query(&literal) {
                    true => query(backend, text, literal.content.clone(), offset),
                    false => backend.location_names(before.matches('.').count()).iter()
                        .map(|name| item(name, CompletionItemKind::CLASS, Some(backend.name())))
                        .collect()
                };

                let known: HashSet<String> = found.iter().map(|item| item.label.clone()).collect();
                found.extend(items.into_iter().filter(|item| !known.contains(&item.label)));
            }
            found
        }
        Context::None => vec![]
    }
}

fn query(backend: &dyn LanguageBackend, text: &str, content: Range<usize>, offset: usize) -> Vec<CompletionItem> {
    let source = &text[content.clone()];
    let Some((part, word)) = part_at(source, offset - content.start) else { return vec![] };
    let range = LineIndex::new(text).range(content.start + word.start..content.start + word.end);
    let separator = if source[word.end..].trim_start().starts_with(':') { "" } else { ": " };

    let names: Vec<(String, String)> = match part {
        Part::Kind => backend.node_kinds().into_iter().map(|kind| (kind, String::new())).collect(),
        Part::Field => backend.fields().into_iter().map(|field| (field, separator.to_string())).collect(),
        Part::Predicate => PREDICATES.iter().map(|(name, _)| (name.to_string(), String::new())).collect(),
        Part::Capture => {
            let mut names = captures(&format!("{}{}", &source[..word.start], &source[word.end..]))
                .into_iter().map(str::to_string).collect::<BTreeSet<_>>();
            names.insert("target".to_string());
            names.into_iter().map(|name| (name, String::new())).collect()
        }
    };
    let kind = match part {
        Part::Kind => CompletionItemKind::CLASS,
        Part::Field => CompletionItemKind::FIELD,
        Part::Predicate => CompletionItemKind::FUNCTION,
        Part::Capture => CompletionItemKind::VARIABLE
    };

    names.into_iter()
        .map(|(name, suffix)| CompletionItem {
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, format!("{name}{suffix}")))),
            ..item(&name, kind, Some(backend.name()))
        })
        .collect()
}

pub fn document(registry: &Registry, path: &Path, text: &str, offset: usize) -> Vec<CompletionItem> {
    let is_settings = kind(path) == DocumentKind::Settings;
    let outline = Outline::parse(text);

    match outline.context(text, offset) {
        Context::Value { node, .. } if is_settings && outline.is_top(node, options::TARGET) => {
            registry.targets().into_iter().map(|name| item(name, CompletionItemKind::VALUE, None)).collect()
        }
        Context::Name { parent: Some(node), prefix } if outline.is_top(node, options::TARGET) => {
            let target = options::target_kind(path, text);
            let known = target.and_then(|name| registry.loader(&name)).map(|loader| loader.options()).unwrap_or_default();

            known.iter()
                .filter(|name| !options::options(&outline).any(|node| node.name == **name && node.name_range.start != prefix.start))
                .map(|name| item(name, CompletionItemKind::PROPERTY, None))
                .collect()
        }
        _ => {
            let found = paths::completion(path, text, offset);

            if found.is_empty() && is_settings { settings(registry, text, offset) } else { found }
        }
    }
}

pub fn settings(registry: &Registry, text: &str, offset: usize) -> Vec<CompletionItem> {
    let outline = Outline::parse(text);
    let names = registry.languages().into_iter().map(|name| item(name, CompletionItemKind::VALUE, None));

    match outline.context(text, offset) {
        Context::Value { node, .. } if outline.is_top(node, "ast-set") => names.chain([item(TARGET, CompletionItemKind::KEYWORD, None)]).collect(),
        Context::Name { parent: Some(node), .. } if outline.is_top(node, "ast-set") => names.collect(),
        _ => vec![]
    }
}

pub fn item(label: &str, kind: CompletionItemKind, detail: Option<&str>) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: detail.map(str::to_string),
        ..CompletionItem::default()
    }
}
