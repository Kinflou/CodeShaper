// Standard Uses
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

// Crate Uses

// External Uses
use miette::{bail, Result};
use regex::{Regex, RegexBuilder};
use regex_syntax::ast::{self, Ast, GroupKind, Visitor};
use regex_syntax::hir::Hir;
use regex_syntax::{Error, ParserBuilder};


pub const FLAGS: [(char, &str); 4] = [
    ('i', "case insensitive"),
    ('m', "'^' and '$' match at line breaks"),
    ('s', "'.' also matches '\\n'"),
    ('x', "whitespace and '#' comments are ignored")
];


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub range: Range<usize>,
    pub name: Option<String>
}

type Compiled = HashMap<(String, String), Option<Rc<Regex>>>;

#[derive(Debug, Default)]
pub struct Regexes {
    compiled: RefCell<Compiled>
}

impl Regexes {
    pub fn get(&self, pattern: &str, flags: &str) -> Option<Rc<Regex>> {
        self.compiled.borrow_mut()
            .entry((pattern.to_string(), flags.to_string()))
            .or_insert_with(|| regex_builder(pattern, flags).build().ok().map(Rc::new))
            .clone()
    }
}


pub fn check_flags(flags: &str) -> Result<()> {
    if let Some(unknown) = flags.chars().find(|flag| !FLAGS.iter().any(|(known, _)| known == flag)) {
        bail!("Unknown regex flag '{unknown}', expected any of 'i', 'm', 's' or 'x'")
    }

    Ok(())
}

pub fn regex_builder(pattern: &str, flags: &str) -> RegexBuilder {
    let mut builder = RegexBuilder::new(pattern);

    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .ignore_whitespace(flags.contains('x'));

    builder
}

pub fn groups(pattern: &str, flags: &str) -> Option<Vec<Group>> {
    let parsed = ast::parse::ParserBuilder::new()
        .ignore_whitespace(flags.contains('x'))
        .build()
        .parse(pattern)
        .ok()?;

    ast::visit(&parsed, Groups::default()).ok()
}

pub fn matches_empty(pattern: &str, flags: &str) -> bool {
    parse(pattern, flags).is_ok_and(|parsed| {
        let properties = parsed.properties();

        properties.minimum_len() == Some(0) && properties.look_set().is_empty()
    })
}

fn parse(pattern: &str, flags: &str) -> Result<Hir, Box<Error>> {
    ParserBuilder::new()
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .ignore_whitespace(flags.contains('x'))
        .build()
        .parse(pattern)
        .map_err(Box::new)
}

pub fn error(pattern: &str, flags: &str) -> Option<(String, Range<usize>)> {
    let error = parse(pattern, flags).err()?;

    let (message, span) = match &*error {
        Error::Parse(error) => (error.kind().to_string(), error.span()),
        Error::Translate(error) => (error.kind().to_string(), error.span()),
        _ => return None
    };

    Some((message, span.start.offset..span.end.offset))
}


#[derive(Default)]
struct Groups {
    found: Vec<Group>
}

impl Visitor for Groups {
    type Output = Vec<Group>;
    type Err = ();

    fn finish(self) -> Result<Vec<Group>, ()> {
        Ok(self.found)
    }

    fn visit_pre(&mut self, ast: &Ast) -> Result<(), ()> {
        let Ast::Group(group) = ast else { return Ok(()) };
        let name = match &group.kind {
            GroupKind::CaptureIndex(_) => None,
            GroupKind::CaptureName { name, .. } => Some(name.name.clone()),
            GroupKind::NonCapturing(_) => return Ok(())
        };

        self.found.push(Group { range: group.span.start.offset..group.span.end.offset, name });
        Ok(())
    }
}
