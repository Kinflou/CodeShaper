// Standard Uses
use std::ops::Range;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::shaping::patch::Actions;

// External Uses
use serde::{Serialize, Deserialize};
use knuffel::Decode;
use miette::{IntoDiagnostic, Result};
use regex::Regex;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Replacer {
    pub location: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub flags: String,
    #[serde(default)]
    pub reference_location: String,
    pub reference: String,
    pub actions: Actions
}

impl Replacer {
    pub fn apply(&self, backend: &dyn LanguageBackend, source: &str) -> Result<String> {
        let mut ranges = backend.locate(source, &self.location)?;
        ranges.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

        let mut outermost: Vec<Range<usize>> = vec![];
        for range in ranges {
            if outermost.last().is_some_and(|last| range.start < last.end) {
                continue
            }
            outermost.push(range);
        }

        let from = Regex::new(&self.from).into_diagnostic()?;
        let to = Regex::new(r"\\g<(\w+)>").unwrap().replace_all(&self.to, "$${${1}}");

        let mut result = source.to_string();
        for range in outermost.into_iter().rev() {
            let replaced = from.replace_all(&source[range.clone()], to.as_ref()).into_owned();
            result.replace_range(range, &replaced);
        }

        Ok(result)
    }
}

