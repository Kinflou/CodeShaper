// Standard Uses
use std::fmt::{Display, Formatter};
use std::ops::Range;

// Crate Uses

// External Uses
use miette::{Diagnostic, Result};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationError {
    pub message: String,
    pub range: Option<Range<usize>>
}

impl Display for LocationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LocationError {}

impl Diagnostic for LocationError {}


pub trait LanguageBackend {
    fn name(&self) -> &str;
    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>>;

    fn check_location(&self, _location: &str) -> Result<()> {
        Ok(())
    }

    fn describe_location(&self, location: &str) -> Result<String> {
        self.check_location(location).map(|_| format!("{} location `{}`", self.name(), location.trim()))
    }

    fn node_kinds(&self) -> Vec<String> {
        vec![]
    }

    fn fields(&self) -> Vec<String> {
        vec![]
    }

    fn location_names(&self, _segment: usize) -> Vec<String> {
        vec![]
    }
}
