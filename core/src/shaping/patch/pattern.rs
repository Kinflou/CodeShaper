// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use regex_syntax::{Error, ParserBuilder};


pub fn error(pattern: &str, flags: &str) -> Option<(String, Range<usize>)> {
    let error = ParserBuilder::new()
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .ignore_whitespace(flags.contains('x'))
        .build()
        .parse(pattern)
        .err()?;

    let (message, span) = match &error {
        Error::Parse(error) => (error.kind().to_string(), error.span()),
        Error::Translate(error) => (error.kind().to_string(), error.span()),
        _ => return None
    };

    Some((message, span.start.offset..span.end.offset))
}
