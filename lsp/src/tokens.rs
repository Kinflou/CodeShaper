// Standard Uses

// Crate Uses
use crate::position::LineIndex;
use crate::syntax::{is_name, Outline};

// External Uses
use lsp_types::{SemanticToken, SemanticTokenType};


pub const LEGEND: [SemanticTokenType; 3] = [
    SemanticTokenType::FUNCTION, SemanticTokenType::VARIABLE, SemanticTokenType::PARAMETER
];


pub fn tokens(text: &str) -> Vec<SemanticToken> {
    let outline = Outline::parse(text);
    let mut found = vec![];

    for (_, literal) in outline.literals().filter(|(_, literal)| literal.quoted) {
        let content = &text[literal.content.clone()];

        for (index, marker) in content.char_indices() {
            let kind = match (marker, content.as_bytes().get(index + 1) == Some(&b'[')) {
                ('#', true) => 0,
                ('$', true) => 1,
                ('@', true) => 2,
                _ => continue
            };

            let start = literal.content.start + index;
            let name = &text[start + 2..literal.content.end];
            let length = name.find(|character: char| !is_name(character)).unwrap_or(name.len());
            let end = start + 2 + length + usize::from(name[length..].starts_with(']'));

            if length > 0 {
                found.push((start, end, kind));
            }
        }
    }

    found.sort();
    encode(text, &found)
}

fn encode(text: &str, found: &[(usize, usize, u32)]) -> Vec<SemanticToken> {
    let index = LineIndex::new(text);
    let mut previous = (0, 0);

    found.iter()
        .map(|(start, end, kind)| {
            let position = index.position(*start);
            let delta_line = position.line - previous.0;
            let delta_start = if delta_line == 0 { position.character - previous.1 } else { position.character };
            previous = (position.line, position.character);

            SemanticToken {
                delta_line,
                delta_start,
                length: text[*start..*end].encode_utf16().count() as u32,
                token_type: *kind,
                token_modifiers_bitset: 0
            }
        })
        .collect()
}
