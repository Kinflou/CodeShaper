// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use miette::Result;


pub trait LanguageBackend {
    fn name(&self) -> &str;
    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>>;
}
