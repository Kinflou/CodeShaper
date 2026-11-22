// Standard Uses
use std::ops::Range;

// Crate Uses
use crate::syntax::{is_name, Outline};

// External Uses


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
            let length = text[start..literal.content.end].find(|character: char| !is_name(character))
                .unwrap_or(literal.content.end - start);

            if length > 0 {
                found.push(Symbol { name: text[start..start + length].to_string(), range: start..start + length });
            }
        }
    }

    found
}

pub fn definitions(outline: &Outline) -> Vec<Symbol> {
    outline.actions()
        .map(|(_, name)| Symbol { name: name.value.clone(), range: name.content.clone() })
        .collect()
}

pub fn name_at(outline: &Outline, text: &str, offset: usize) -> Option<String> {
    calls(outline, text).into_iter()
        .chain(definitions(outline))
        .find(|symbol| symbol.range.start <= offset && offset <= symbol.range.end)
        .map(|symbol| symbol.name)
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

