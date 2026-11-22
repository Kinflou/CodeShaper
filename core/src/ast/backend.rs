// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use miette::Result;


pub trait LanguageBackend {
    fn name(&self) -> &str;
    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>>;

    fn check_location(&self, _location: &str) -> Result<()> {
        Ok(())
    }

    fn describe_location(&self, location: &str) -> Result<String> {
        self.check_location(location).map(|_| format!("{} location `{}`", self.name(), location.trim()))
    }

    fn location_names(&self, _segment: usize) -> Vec<String> {
        vec![]
    }
}
