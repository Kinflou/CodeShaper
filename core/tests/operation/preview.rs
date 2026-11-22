// Standard Uses
use std::path::PathBuf;

// Crate Uses
use crate::operation::{registry, rename, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;


#[test]
fn preview_runs_one_patch_on_matching_files() {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("rename.kdl", &rename("lines", "Line").replace("enabled true", "enabled false")).unwrap();

    let changes = operation.preview(&registry(), "rename.kdl", &patch).unwrap();
    let found: Vec<(PathBuf, &str, String)> = changes.iter()
        .map(|change| (change.file.clone(), change.before.as_str(), change.result.as_ref().unwrap().clone()))
        .collect();

    assert_eq!(found, vec![
        (PathBuf::from("a.txt"), "old one\nkeep\n", "new one\nkeep\n".to_string()),
        (PathBuf::from("nested/b.txt"), "keep\n", "keep\n".to_string()),
    ]);
    assert_eq!(changes[0].patch, "rename.kdl");
}

#[test]
fn preview_keeps_failures_per_file() {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("broken.kdl", &rename("lines", "Missing")).unwrap();

    let changes = operation.preview(&registry(), "broken.kdl", &patch).unwrap();

    assert_eq!(changes.len(), 2);
    assert!(changes.iter().all(|change| change.result.is_err()));
}

#[test]
fn invalid_file_patterns_are_errors() {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("bad.kdl", &rename("lines", "Line").replace("**/*.txt", "[")).unwrap();

    assert!(operation.preview(&registry(), "bad.kdl", &patch).is_err());
}
