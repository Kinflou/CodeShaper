// Standard Uses
use std::path::PathBuf;

// Crate Uses

// External Uses


#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub files: Vec<FileReport>
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileReport {
    pub path: PathBuf,
    pub outcome: Outcome
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Unchanged,
    Patched(Vec<String>)
}
