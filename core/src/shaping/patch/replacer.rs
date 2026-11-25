// Standard Uses

// Crate Uses
use crate::shaping::patch::Actions;
use crate::shaping::patch::pattern::{check_flags, regex_builder};

// External Uses
use knus::Decode;
use miette::{bail, IntoDiagnostic, Result};
use regex::Regex;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct Replacer {
    #[knus(argument)]
    pub name: Option<String>,
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
    #[knus(child, default)]
    pub actions: Actions
}

impl Replacer {
    pub fn label(name: Option<&str>, index: usize) -> String {
        match name.filter(|name| Self::check_name(name).is_ok()) {
            Some(name) => name.to_string(),
            None => format!("replacer #{}", index + 1)
        }
    }

    pub fn check_name(name: &str) -> Result<()> {
        if name.trim().is_empty() {
            bail!("The replacer's name is empty")
        }
        if name.contains(|character: char| matches!(character, '/' | '#') || character.is_control()) {
            bail!("A replacer's name can't have '/', '#' or control characters")
        }

        Ok(())
    }

    pub fn target_location(&self) -> &str {
        if self.reference_location.is_empty() { &self.location } else { &self.reference_location }
    }

    pub fn from_regex(&self) -> Result<Regex> {
        check_flags(&self.flags)?;

        regex_builder(&self.from, &self.flags).build().into_diagnostic()
    }
}
