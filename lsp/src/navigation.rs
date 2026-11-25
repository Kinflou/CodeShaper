// Standard Uses
use std::ops::Range;

// Crate Uses
use crate::syntax::{is_call, is_name, Outline, ACTION_NODES};

// External Uses
use codeshaper::core::expressions::builtins::is_builtin;
use lsp_types::{GotoDefinitionResponse, Location, LocationLink};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub range: Range<usize>
}


pub fn calls(outline: &Outline, text: &str) -> Vec<Symbol> {
    let mut found = vec![];

    for (_, literal) in outline.literals().filter(|(_, literal)| literal.quoted) {
        let content = &text[literal.content.clone()];

        for (index, _) in content.match_indices("#[") {
            let start = literal.content.start + index + 2;
            let content = &text[start..literal.content.end];
            let length = content.find(|character: char| !is_call(character)).unwrap_or(content.len());

            if length > 0 {
                found.push(Symbol { name: text[start..start + length].to_string(), range: start..start + length });
            }
        }
    }

    found
}

pub fn definitions(outline: &Outline) -> Vec<Symbol> {
    outline.nodes.iter()
        .filter(|node| ACTION_NODES.contains(&node.name.as_str()))
        .filter_map(|node| node.arguments.first())
        .map(|name| Symbol { name: name.value.clone(), range: name.content.clone() })
        .collect()
}

pub fn name_at(outline: &Outline, text: &str, offset: usize) -> Option<String> {
    symbol_at(outline, text, offset).map(|symbol| symbol.name)
}

pub fn definition(outline: &Outline, text: &str, offset: usize) -> Option<Range<usize>> {
    let name = name_at(outline, text, offset)?;

    definitions(outline).into_iter().find(|symbol| symbol.name == name).map(|symbol| symbol.range)
}

pub fn references(outline: &Outline, text: &str, offset: usize, declaration: bool) -> Vec<Range<usize>> {
    let Some(name) = name_at(outline, text, offset) else { return vec![] };

    let declared = definitions(outline).into_iter().filter(|_| declaration);

    declared.chain(calls(outline, text))
        .filter(|symbol| symbol.name == name)
        .map(|symbol| symbol.range)
        .collect()
}


pub fn symbol_at(outline: &Outline, text: &str, offset: usize) -> Option<Symbol> {
    calls(outline, text).into_iter()
        .chain(definitions(outline))
        .find(|symbol| symbol.range.start <= offset && offset <= symbol.range.end)
}

pub fn rename(outline: &Outline, text: &str, offset: usize, name: &str) -> Option<Vec<Range<usize>>> {
    if name.is_empty() || !name.chars().all(is_name) { return None }
    symbol_at(outline, text, offset).filter(|symbol| !is_builtin(&symbol.name))?;

    Some(references(outline, text, offset, true))
}

pub fn links(wanted: bool, origin: lsp_types::Range, locations: Vec<Location>) -> GotoDefinitionResponse {
    if !wanted { return GotoDefinitionResponse::Array(locations) }

    GotoDefinitionResponse::Link(locations.into_iter()
        .map(|location| LocationLink {
            origin_selection_range: Some(origin),
            target_uri: location.uri,
            target_range: location.range,
            target_selection_range: location.range
        })
        .collect())
}
