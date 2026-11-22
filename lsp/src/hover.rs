// Standard Uses
use std::ops::Range;
use std::path::Path;

// Crate Uses
use crate::completion::backend;
use crate::navigation::name_at;
use crate::preview::configuration;
use crate::schema::LOCATIONS;
use crate::syntax::{Context, Outline};

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::{LocationMatches, Operation};
use codeshaper::core::shaping::patch::Patch;


pub fn hover(registry: &Registry, project: &Path, text: &str, offset: usize) -> Option<(String, Range<usize>)> {
    let outline = Outline::parse(text);

    if let Some(name) = name_at(&outline, text, offset) {
        let (node, literal) = outline.actions().find(|(_, literal)| literal.value == name)?;
        let end = node.children.as_ref().map(|children| (children.end + 1).min(text.len())).unwrap_or(literal.range.end);

        return Some((format!("```kdl\n{}\n```", &text[node.name_range.start..end]), literal.range.clone()))
    }

    let Context::Value { node, literal, .. } = outline.context(text, offset) else { return None };
    if !LOCATIONS.contains(&outline.nodes[node].name.as_str()) { return None }

    let backend = backend(registry, project, &outline)?;
    let description = match backend.describe_location(&literal.value) {
        Ok(description) => description,
        Err(error) => return Some((format!("Not a {} location: {error}", backend.name()), literal.range))
    };

    let found = matches(registry, project, &outline, &literal.value).map(|matches| format!(
        "\n\nMatches {} place(s) in {} of {} target file(s)", matches.places, matches.matched_files, matches.files
    ));

    Some((description + &found.unwrap_or_default(), literal.range))
}

fn matches(registry: &Registry, project: &Path, outline: &Outline, location: &str) -> Option<LocationMatches> {
    let patch = Patch {
        alias: outline.argument("alias").unwrap_or_default().to_string(),
        file: outline.argument("file").unwrap_or_default().to_string(),
        ..Patch::default()
    };
    let operation = Operation::from_file(&configuration(project).ok()?).ok()?;

    operation.locate(registry, "", &patch, location).ok()
}
