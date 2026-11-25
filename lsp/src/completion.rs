// Standard Uses
use std::collections::{BTreeSet, HashSet};
use std::ops::Range;
use std::path::Path;

// Crate Uses
use crate::document::{kind, DocumentKind};
use crate::explain::{self, documentation, Explained};
use crate::groups::groups;
use crate::options;
use crate::paths;
use crate::position::LineIndex;
use crate::query::{captures, is_query, part_at, Part, PREDICATES};
use crate::schema::{self, regex_field, LOCATIONS};
use crate::section::{view_at, View};
use crate::syntax::{is_call, is_name, Context, Outline};
use crate::target::{backends, every_file};
use crate::values;

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::expressions::builtins::{is_builtin, BUILTINS};
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::patch::file::BLOCK;
use codeshaper::core::shaping::project::ast_set::TARGET;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, MarkupContent, MarkupKind, TextEdit};


pub fn completion(registry: &Registry, project: &Path, text: &str, offset: usize) -> Vec<CompletionItem> {
    let View { section, text } = view_at(text, offset);
    let text = text.as_ref();
    let outline = Outline::parse(text);

    match outline.context(text, offset) {
        Context::Name { parent: None, .. } if section.is_none() => {
            let has = |names: &[&str]| outline.nodes.iter().any(|node| node.parent.is_none() && names.contains(&node.name.as_str()));

            schema::top(has(&[BLOCK]), has(schema::children(None))).into_iter()
                .map(|name| named(explain::PATCH, None, name))
                .collect()
        }
        Context::Name { parent, .. } => {
            let parent = parent.map(|parent| outline.nodes[parent].name.as_str());

            schema::children(parent).iter()
                .map(|name| named(explain::PATCH, parent, name))
                .collect()
        }
        Context::Value { node, literal, before } => {
            if outline.is_top(node, "file") {
                return paths::target(text, &literal.content, offset, &every_file(registry, project))
            }

            if outline.is_top(node, "alias") {
                return registry.languages().into_iter().map(|name| item(name, CompletionItemKind::VALUE, None)).collect()
            }

            if let Some((_, name)) = before.rsplit_once("#[").filter(|(_, name)| name.chars().all(is_call)) {
                let builtins = builtins(text, offset - name.len()..offset);
                if is_builtin(name) { return builtins.collect() }

                return outline.actions()
                    .map(|(node, name)| item(&name.value, CompletionItemKind::FUNCTION, Some(&node.name)))
                    .chain(values::suggestions(project).map(|(name, detail, kind)| item(&name, kind, Some(detail))))
                    .chain(builtins)
                    .collect()
            }

            if before.rsplit_once("$[").is_some_and(|(_, name)| name.chars().all(is_name)) {
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

fn builtins(text: &str, range: Range<usize>) -> impl Iterator<Item = CompletionItem> {
    let range = LineIndex::new(text).range(range);

    BUILTINS.iter().map(move |builtin| CompletionItem {
        label: builtin.name.to_string(),
        kind: Some(if builtin.arguments.is_empty() { CompletionItemKind::CONSTANT } else { CompletionItemKind::FUNCTION }),
        detail: Some("built-in".to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: explain::builtin(builtin) })),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, builtin.name.to_string()))),
        ..CompletionItem::default()
    })
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
                .map(|name| named(explain::table(path), Some(options::TARGET), name))
                .collect()
        }
        _ => {
            let found = paths::completion(path, text, offset);

            match kind(path) {
                DocumentKind::Settings if found.is_empty() => settings(registry, text, offset),
                DocumentKind::Configuration if found.is_empty() => values::completion(path, text, offset),
                _ => found
            }
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

fn named(table: &[Explained], parent: Option<&str>, name: &str) -> CompletionItem {
    CompletionItem { documentation: documentation(table, parent, name), ..item(name, CompletionItemKind::PROPERTY, None) }
}

pub fn item(label: &str, kind: CompletionItemKind, detail: Option<&str>) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: detail.map(str::to_string),
        ..CompletionItem::default()
    }
}
