// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses


#[derive(Debug, Clone, PartialEq, Eq)]
struct Part {
    range: Range<usize>,
    origin: usize
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    parts: Vec<Part>,
    returns: Vec<usize>
}

impl Origin {
    pub fn new(source: &str) -> Self {
        let returns: Vec<usize> = source.match_indices("\r\n").enumerate()
            .map(|(earlier, (index, _))| index - earlier)
            .collect();
        let length = source.len() - returns.len();

        Self { parts: vec![Part { range: 0..length, origin: 0 }], returns }
    }

    pub fn edit<'t>(&mut self, edits: impl Iterator<Item = (usize, &'t str, &'t str)>) {
        let mut parts = std::mem::take(&mut self.parts).into_iter().peekable();
        let mut shift = 0;
        let changes = edits.filter(|(_, before, after)| before != after).map(|(start, before, after)| change(start, before, after));

        for (removed, inserted) in changes.chain(std::iter::once((usize::MAX..usize::MAX, 0))) {
            while let Some(part) = parts.peek_mut() {
                let end = part.range.end.min(removed.start);
                if part.range.start < end {
                    self.keep(Part { range: shifted(part.range.start, shift)..shifted(end, shift), origin: part.origin });
                }

                if part.range.end <= removed.end { parts.next(); continue }

                let start = part.range.start.max(removed.end);
                part.origin += start - part.range.start;
                part.range.start = start;
                break
            }

            shift += inserted as isize - removed.len() as isize;
        }
    }

    fn keep(&mut self, part: Part) {
        match self.parts.last_mut() {
            Some(last) if last.range.end == part.range.start && last.origin + last.range.len() == part.origin =>
                last.range.end = part.range.end,
            _ => self.parts.push(part)
        }
    }

    pub fn place(&self, range: Range<usize>) -> Option<Range<usize>> {
        let index = self.parts.partition_point(|part| part.range.end < range.start);
        let part = self.parts[index..].iter().take(2).find(|part| part.range.start <= range.start && range.end <= part.range.end)?;
        let place = |offset: usize| {
            let offset = part.origin + offset - part.range.start;

            offset + self.returns.partition_point(|index| *index < offset)
        };

        Some(place(range.start)..place(range.end))
    }
}


fn change(start: usize, before: &str, after: &str) -> (Range<usize>, usize) {
    let prefix = before.bytes().zip(after.bytes()).take_while(|(old, new)| old == new).count();
    let suffix = before.bytes().rev().zip(after.bytes().rev())
        .take(before.len().min(after.len()) - prefix)
        .take_while(|(old, new)| old == new)
        .count();

    (start + prefix..start + before.len() - suffix, after.len() - suffix - prefix)
}

fn shifted(offset: usize, shift: isize) -> usize {
    (offset as isize + shift) as usize
}
