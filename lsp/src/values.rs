// Standard Uses
use std::collections::BTreeMap;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

// Crate Uses
use crate::options::project;
use crate::position::LineIndex;
use crate::preview::configuration;
use crate::syntax::{is_name, Outline};
use crate::trace::fenced;

// External Uses
use codeshaper::core::shaping::operation::configuration::OperationConfiguration;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::project::settings::{self, ProjectSettings};
use codeshaper::core::shaping::project::values::{self, check_settable, Declared, Source};
use dunce::canonicalize;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, TextEdit};
use miette::{IntoDiagnostic, Result};
use serde_json::{json, Value};


pub const KINDS: [&str; 2] = ["constant", "variable"];
pub const SET: &str = "variables";

type Given = BTreeMap<String, String>;

static GIVEN: Mutex<BTreeMap<PathBuf, Given>> = Mutex::new(BTreeMap::new());


fn all_given() -> MutexGuard<'static, BTreeMap<PathBuf, Given>> {
    GIVEN.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn give(project: &Path, values: Given) {
    if let Ok(project) = canonicalize(project) { all_given().insert(project, values); }
}

pub fn given(project: &Path) -> Given {
    canonicalize(project).ok().and_then(|project| all_given().get(&project).cloned()).unwrap_or_default()
}

pub fn prepared(mut operation: Operation) -> Operation {
    for (name, value) in given(Path::new(&operation.project.directory)) {
        let _ = operation.set(&name, &value);
    }

    operation
}

pub fn declared(project: &Path) -> Vec<Declared> {
    let Ok(settings) = ProjectSettings::from_path(project) else { return vec![] };
    if settings.constants.is_empty() && settings.variables.is_empty() { return vec![] }

    values::declared(&settings, &configured(project), &given(project))
}

fn configured(project: &Path) -> BTreeMap<String, String> {
    configuration(project).ok()
        .and_then(|path| OperationConfiguration::from_file(&path).ok())
        .map(|configuration| configuration.variables)
        .unwrap_or_default()
}

pub fn listed(project: &Path) -> Result<Value> {
    let settings = ProjectSettings::from_path(project)?;
    let text = fs::read_to_string(file(project)).into_diagnostic()?;
    let index = LineIndex::new(&text);
    let (configured, given) = (configured(project), given(project));
    let unset = values::declared(&settings, &configured, &BTreeMap::new());

    let variables: Vec<Value> = values::declared(&settings, &configured, &given).into_iter().zip(unset)
        .map(|(declared, unset)| json!({
            "name": declared.name,
            "kind": declared.kind(),
            "value": declared.value,
            "source": source(declared.source),
            "otherwise": unset.value,
            "range": declaration(&text, &declared.name).map(|range| index.range(range))
        }))
        .collect();
    let unknown: Vec<&String> = given.keys().filter(|name| check_settable(&settings, name).is_err()).collect();

    Ok(json!({ "name": settings.name, "variables": variables, "unknown": unknown }))
}

fn source(source: Source) -> &'static str {
    match source {
        Source::Constant => "constant",
        Source::Set => "set",
        Source::Configuration => "configuration",
        Source::Default => "default",
        Source::Missing => "missing"
    }
}

pub fn find(project: &Path, name: &str) -> Option<Declared> {
    declared(project).into_iter().find(|declared| declared.name == name)
}

pub fn file(project: &Path) -> PathBuf {
    project.join(settings::FILE)
}

pub fn declaration(text: &str, name: &str) -> Option<Range<usize>> {
    let outline = Outline::parse(text);
    let is_declared = |kind: &str| KINDS.contains(&kind);

    outline.nodes.iter()
        .filter(|node| node.parent.is_none() && !node.commented && is_declared(&node.name))
        .filter_map(|node| node.arguments.first())
        .find(|literal| literal.value == name)
        .map(|literal| literal.content.clone())
}

pub fn hover(project: &Path, name: &str) -> Option<String> {
    let found = find(project, name)?;
    let source = match found.source {
        Source::Constant => "Constant of the project",
        Source::Set => "Variable of the project, as set",
        Source::Configuration => "Variable of the project, from the configuration",
        Source::Default => "Variable of the project, its default",
        Source::Missing => "Variable of the project, without a value"
    };

    Some(match found.value.as_deref() {
        None => source.to_string(),
        Some("") => format!("{source}: empty"),
        Some(value) => format!("{source}\n{}", fenced("", value))
    })
}

pub fn suggestions(project: &Path) -> impl Iterator<Item = (String, &'static str, CompletionItemKind)> {
    declared(project).into_iter().map(|declared| {
        let kind = match declared.source {
            Source::Constant => CompletionItemKind::CONSTANT,
            _ => CompletionItemKind::VARIABLE
        };

        (declared.name.clone(), declared.kind(), kind)
    })
}

pub fn completion(path: &Path, text: &str, offset: usize) -> Vec<CompletionItem> {
    let outline = Outline::parse(text);
    let start = text[..offset].trim_end_matches(is_name).len();
    let Some(node) = outline.top(SET).filter(|node| node.name_range.end < offset) else { return vec![] };
    let between = &text[node.name_range.end..start];
    let is_value = outline.literals().any(|(_, literal)| literal.quoted && literal.range.start < offset && offset < literal.range.end);
    if is_value || between.contains(['\n', ';', '{']) || !between.ends_with(char::is_whitespace) { return vec![] }

    let Some(settings) = project(path, text).and_then(|project| ProjectSettings::from_path(&project).ok()) else { return vec![] };
    let range = LineIndex::new(text).range(start..offset);

    settings.variables.into_iter()
        .filter(|variable| !node.properties.iter().any(|(name, _)| *name == variable.name))
        .map(|variable| CompletionItem {
            label: variable.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: variable.default.map(|default| format!("default {default}")),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, format!("{}=\"\"", variable.name)))),
            ..CompletionItem::default()
        })
        .collect()
}

pub fn wrong(path: &Path, text: &str) -> Vec<(Range<usize>, String)> {
    let outline = Outline::parse(text);
    let Some(node) = outline.top(SET) else { return vec![] };
    let Some(settings) = project(path, text).and_then(|project| ProjectSettings::from_path(&project).ok()) else { return vec![] };

    node.properties.iter()
        .filter_map(|(name, value)| {
            let error = check_settable(&settings, name).err()?;
            let key = text[..value.range.start].strip_suffix('=').filter(|key| key.ends_with(name.as_str()));
            let range = key.map(|key| key.len() - name.len()..key.len()).unwrap_or(value.range.clone());

            Some((range, error.to_string()))
        })
        .collect()
}
