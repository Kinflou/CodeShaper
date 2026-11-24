// Standard Uses
use std::collections::HashMap;
use std::sync::Arc;

// Crate Uses
use crate::groups::links;
use crate::position::LineIndex;
use crate::schema::{regex_field, result_field, LOCATIONS};
use crate::section::views;
use crate::syntax::{Outline, ACTION_NODES};

// External Uses
use codeshaper::core::shaping::operation::LocationMatches;
use codeshaper::core::shaping::patch::trace::{ActionTrace, Trace};
use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel};


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Hints {
    pub groups: bool,
    pub matches: bool
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub alias: String,
    pub file: String,
    pub location: String
}

pub type Counts = HashMap<Place, LocationMatches>;


pub fn places(outline: &Outline) -> Vec<(Place, usize)> {
    outline.literals()
        .filter(|(node, literal)| literal.quoted && LOCATIONS.contains(&outline.nodes[*node].name.as_str()))
        .map(|(_, literal)| {
            let place = Place {
                alias: outline.argument("alias").unwrap_or_default().to_string(),
                file: outline.argument("file").unwrap_or_default().to_string(),
                location: literal.value.clone()
            };

            (place, literal.range.end)
        })
        .collect()
}

pub fn inlay_hints(text: &str, hints: Hints, counts: &Counts, traces: &[Option<Arc<Trace>>]) -> Vec<InlayHint> {
    let index = LineIndex::new(text);
    let mut found: Vec<InlayHint> = views(text).iter().enumerate()
        .flat_map(|(number, view)| {
            let trace = traces.get(number).and_then(|trace| trace.as_deref());

            view_hints(&view.text, &index, hints, counts, trace)
        })
        .collect();

    found.sort_by_key(|hint| hint.position);
    found
}

fn view_hints(text: &str, index: &LineIndex, hints: Hints, counts: &Counts, trace: Option<&Trace>) -> Vec<InlayHint> {
    let outline = Outline::parse(text);
    let mut found = vec![];

    if hints.groups {
        for (action, _) in outline.nodes.iter().enumerate().filter(|(_, node)| ACTION_NODES.contains(&node.name.as_str())) {
            for (number, link) in links(&outline, text, action).unwrap_or_default().into_iter().enumerate() {
                found.push(hint(index, link.group.start, format!("{number}:"), Some(InlayHintKind::PARAMETER)));
            }
        }
    }

    if hints.matches {
        for (place, end) in places(&outline) {
            let Some(matches) = counts.get(&place) else { continue };
            let label = format!("{} in {}", counted(matches.places, "place"), counted(matches.matched_files, "file"));

            found.push(after(index, end, label));
        }

        if let Some(trace) = trace.filter(|trace| trace.files > 0) {
            found.extend(results(&outline, trace).into_iter().map(|(end, label)| after(index, end, label)));
        }
    }

    found
}

fn results(outline: &Outline, trace: &Trace) -> Vec<(usize, String)> {
    let mut found = vec![];

    for (action, node) in outline.nodes.iter().enumerate().filter(|(_, node)| ACTION_NODES.contains(&node.name.as_str())) {
        let Some(path) = outline.action_path(action) else { continue };
        let traced = trace.action(&path).cloned().unwrap_or_default();
        let end = |field: &str| outline.field(action, field).map(|literal| literal.range.end);

        let regex = regex_field(&node.name).and_then(end).zip(matched(&node.name, &traced));
        let result = result_field(&node.name).and_then(end).map(|end| (end, resulted(&node.name, &traced)));
        found.extend(regex.into_iter().chain(result));
    }

    found
}

fn matched(kind: &str, traced: &ActionTrace) -> Option<String> {
    if traced.inputs == 0 { return None }

    Some(match kind {
        "replacer" => format!("{} in {} of {}", counted_as(traced.matches, "match", "matches"), traced.matched, traced.inputs),
        _ => format!("{} of {} match", traced.matched, traced.inputs)
    })
}

fn resulted(kind: &str, traced: &ActionTrace) -> String {
    match kind {
        "builder" if traced.calls > 0 => format!("{} built in {}", traced.built, counted(traced.calls, "call")),
        "builder" if traced.inputs > 0 => format!("{} built", traced.built),
        "builder" => "never built".to_string(),
        _ if traced.calls == 0 => "not called".to_string(),
        _ => counted(traced.calls, "call")
    }
}

fn after(index: &LineIndex, end: usize, label: String) -> InlayHint {
    InlayHint { padding_left: Some(true), ..hint(index, end, label, None) }
}

fn hint(index: &LineIndex, offset: usize, label: String, kind: Option<InlayHintKind>) -> InlayHint {
    InlayHint {
        position: index.position(offset),
        label: InlayHintLabel::String(label),
        kind,
        text_edits: None,
        tooltip: None,
        padding_left: None,
        padding_right: None,
        data: None
    }
}

fn counted(count: usize, name: &str) -> String {
    counted_as(count, name, &format!("{name}s"))
}

fn counted_as(count: usize, one: &str, several: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { several })
}
