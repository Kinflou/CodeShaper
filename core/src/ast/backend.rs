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


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLanguage {
    pub name: &'static str,
    pub documentation: &'static str
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxNode {
    pub kind: String,
    pub field: Option<String>,
    pub range: Range<usize>,
    pub parent: Option<usize>,
    pub broken: bool
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub location: String,
    pub ranges: Vec<Range<usize>>
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Suggestions {
    pub paths: Vec<Suggestion>,
    pub query: Option<Suggestion>
}


pub trait LanguageBackend {
    fn name(&self) -> &str;
    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>>;

    fn locate_all(&self, source: &str, locations: &[&str]) -> Vec<Result<Vec<Range<usize>>>> {
        locations.iter().map(|location| self.locate(source, location)).collect()
    }

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

    fn query_language(&self) -> Option<QueryLanguage> {
        None
    }

    fn nodes(&self, _source: &str) -> Result<Vec<SyntaxNode>> {
        Ok(vec![])
    }

    fn suggest_locations(&self, _source: &str, _node: usize) -> Result<Suggestions> {
        Ok(Suggestions::default())
    }
}
