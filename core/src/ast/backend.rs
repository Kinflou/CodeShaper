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

    fn location_names(&self, _segment: usize) -> Vec<String> {
        vec![]
    }
}
