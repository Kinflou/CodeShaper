// Standard Uses

// Crate Uses
use crate::shaping::patch::Actions;

// External Uses
use knus::Decode;
use miette::{bail, IntoDiagnostic, Result};
use regex::{Regex, RegexBuilder};


pub const FLAGS: [(char, &str); 4] = [
    ('i', "case insensitive"),
    ('m', "'^' and '$' match at line breaks"),
    ('s', "'.' also matches '\\n'"),
    ('x', "whitespace and '#' comments are ignored")
];


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct Replacer {
    #[knus(child, unwrap(argument))]
    pub location: String,
    #[knus(child, unwrap(argument))]
    pub from: String,
    #[knus(child, unwrap(argument))]
    pub to: String,
    #[knus(child, unwrap(argument), default)]
    pub flags: String,
    #[knus(child, unwrap(argument), default)]
    pub reference_location: String,
    #[knus(child, unwrap(argument), default)]
    pub reference: String,
    #[knus(child, default)]
    pub actions: Actions
}

impl Replacer {
    pub fn target_location(&self) -> &str {
        if self.reference_location.is_empty() { &self.location } else { &self.reference_location }
    }

    pub fn from_regex(&self) -> Result<Regex> {
        check_flags(&self.flags)?;

        regex_builder(&self.from, &self.flags).build().into_diagnostic()
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
