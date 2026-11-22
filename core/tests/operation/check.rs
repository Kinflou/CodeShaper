// Standard Uses
use std::path::Path;

// Crate Uses
use crate::operation::{patch, registry, rename, run, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::validation::{DiagnosticKind, Severity};
use tempfile::TempDir;


fn check(directory: &TempDir) -> Vec<(String, DiagnosticKind, Severity)> {
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

    operation.check(&registry()).unwrap().into_iter()
        .map(|(name, diagnostic)| (name, diagnostic.kind, diagnostic.severity))
        .collect()
}


#[test]
fn clean_operation() {
    let directory = workspace("replace", &[("rename.kdl", rename("lines", "Line"))]);

    assert_eq!(check(&directory), vec![]);
}

#[test]
fn disabled_patches_are_checked_but_do_not_block() {
    let broken = rename("lines", "Line").replace("enabled true", "enabled false").replace("new $[0]", "#[missing]");
    let directory = workspace("replace", &[("broken.kdl", broken), ("rename.kdl", rename("lines", "Line"))]);

    assert_eq!(check(&directory), vec![
        ("broken.kdl".to_string(), DiagnosticKind::ActionNotFound, Severity::Error)
    ]);

    run(&directory).unwrap();
}

#[test]
fn unmatched_file_pattern() {
    let rust = patch("lines", "Line", "old", "new").replace("**/*.txt", "**/*.rs");
    let directory = workspace("replace", &[("rust.kdl", rust)]);

    assert_eq!(check(&directory), vec![
        ("rust.kdl".to_string(), DiagnosticKind::FileNotMatched, Severity::Warning)
    ]);
}

#[test]
fn unmatched_file_pattern_points_at_file() {
    let rust = patch("lines", "Line", "old", "new").replace("**/*.txt", "**/*.rs");
    let directory = workspace("replace", &[("rust.kdl", rust.clone())]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

    let found = operation.check(&registry()).unwrap();
    let span = found[0].1.span.unwrap();

    assert_eq!(&rust[span.offset()..span.offset() + span.len()], r#""**/*.rs""#);
}

#[test]
fn missing_configuration_is_an_error() {
    let error = Operation::from_file(Path::new("tests/data/missing/configuration.kdl")).err().unwrap();

    assert!(format!("{error:?}").contains("Could not load operation configuration"));
}
