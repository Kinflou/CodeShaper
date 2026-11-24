// Standard Uses
use std::collections::HashMap;

// Crate Uses
use crate::groups::links;
use crate::position::LineIndex;
use crate::schema::LOCATIONS;
use crate::section::views;
use crate::syntax::{Outline, ACTION_NODES};

// External Uses
use codeshaper::core::shaping::operation::LocationMatches;
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

pub fn inlay_hints(text: &str, hints: Hints, counts: &Counts) -> Vec<InlayHint> {
    let index = LineIndex::new(text);
    let mut found: Vec<InlayHint> = views(text).iter().flat_map(|view| view_hints(&view.text, &index, hints, counts)).collect();

    found.sort_by_key(|hint| hint.position);
    found
}

fn view_hints(text: &str, index: &LineIndex, hints: Hints, counts: &Counts) -> Vec<InlayHint> {
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

            found.push(InlayHint { padding_left: Some(true), ..hint(index, end, label, None) });
        }
    }

    found
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
    format!("{count} {name}{}", if count == 1 { "" } else { "s" })
}
