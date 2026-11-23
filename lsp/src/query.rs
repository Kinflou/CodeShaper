// Standard Uses
use std::collections::BTreeSet;
use std::ops::Range;

// Crate Uses
use crate::syntax::Literal;

// External Uses


pub const PREDICATES: [(&str, &str); 10] = [
    ("eq?", "the capture's text equals a string or another capture"),
    ("not-eq?", "the capture's text differs from a string or another capture"),
    ("any-eq?", "any captured node's text equals a string or another capture"),
    ("any-not-eq?", "any captured node's text differs from a string or another capture"),
    ("match?", "the capture's text matches a regex"),
    ("not-match?", "the capture's text doesn't match a regex"),
    ("any-match?", "any captured node's text matches a regex"),
    ("any-not-match?", "any captured node's text doesn't match a regex"),
    ("any-of?", "the capture's text is one of the strings"),
    ("not-any-of?", "the capture's text is none of the strings")
];


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Kind,
    Field,
    Predicate,
    Capture
}


pub fn is_query(literal: &Literal) -> bool {
    literal.value.trim_start().starts_with('(')
}

pub fn part_at(query: &str, offset: usize) -> Option<(Part, Range<usize>)> {
    if is_text(query, offset) { return None }

    let start = query[..offset].trim_end_matches(is_word).len();
    let end = query[offset..].find(|character| !is_word(character)).map(|length| offset + length).unwrap_or(query.len());
    let before = &query[..start];

    let part = match before.chars().next_back() {
        Some('#') => Part::Predicate,
        Some('@') => Part::Capture,
        _ if query[end..].trim_start().starts_with(':') => Part::Field,
        _ if before.trim_end().ends_with('(') => Part::Kind,
        _ => Part::Field
    };

    Some((part, start..end))
}

pub fn captures(query: &str) -> BTreeSet<&str> {
    query.match_indices('@')
        .filter(|(index, _)| !is_text(query, *index))
        .map(|(index, _)| {
            let name = &query[index + 1..];
            &name[..name.find(|character| !is_word(character)).unwrap_or(name.len())]
        })
        .filter(|name| !name.is_empty())
        .collect()
}

fn is_text(query: &str, offset: usize) -> bool {
    let mut string = false;
    let mut comment = false;
    let mut characters = query[..offset].chars();

    while let Some(character) = characters.next() {
        match character {
            '\n' => comment = false,
            _ if comment => {}
            '\\' if string => { characters.next(); }
            '"' => string = !string,
            ';' if !string => comment = true,
            _ => {}
        }
    }

    string || comment
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.' | '?' | '!')
}
