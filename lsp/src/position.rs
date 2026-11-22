// Standard Uses
use std::ops::Range as Span;

// Crate Uses

// External Uses
use lsp_types::{Position, Range};


pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>
}

impl<'a> LineIndex<'a> {
    pub fn new(text: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(index, _)| index + 1))
            .collect();

        Self { text, starts }
    }

    pub fn position(&self, offset: usize) -> Position {
        let offset = self.floor(offset.min(self.text.len()));
        let line = self.starts.partition_point(|start| *start <= offset) - 1;
        let character = self.text[self.starts[line]..offset].encode_utf16().count();

        Position::new(line as u32, character as u32)
    }

    pub fn offset(&self, position: Position) -> usize {
        let Some(start) = self.starts.get(position.line as usize).copied() else { return self.text.len() };
        let end = self.starts.get(position.line as usize + 1).copied().unwrap_or(self.text.len());

        let mut units = 0;
        for (index, character) in self.text[start..end].char_indices() {
            if units >= position.character as usize || character == '\n' {
                return start + index
            }
            units += character.len_utf16();
        }

        end
    }

    pub fn range(&self, span: Span<usize>) -> Range {
        Range::new(self.position(span.start), self.position(span.end))
    }

    fn floor(&self, mut offset: usize) -> usize {
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }

        offset
    }
}
