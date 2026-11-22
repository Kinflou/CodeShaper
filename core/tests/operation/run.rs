// Standard Uses

// Crate Uses
use crate::operation::{patch, read, rename, report, run, workspace};

// External Uses
use codeshaper_core::shaping::operation::report::Outcome;


#[test]
fn create_new_writes_output() {
    let directory = workspace("create-new", &[("rename.kdl", rename("", "Line"))]);

    assert_eq!(run(&directory).unwrap(), vec![
        report("a.txt", Outcome::Patched(vec!["rename.kdl".to_string()])),
        report("nested/b.txt", Outcome::Unchanged),
        report("c.md", Outcome::Unchanged),
    ]);
    assert_eq!(read(&directory, "output/a.txt"), "new one\nkeep\n");
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
    assert!(!directory.path().join("output/nested/b.txt").exists());
    assert!(!directory.path().join("output/c.md").exists());
}

#[test]
fn replace_writes_in_place() {
    let directory = workspace("replace", &[("rename.kdl", rename("lines", "Line"))]);

    run(&directory).unwrap();

    assert_eq!(read(&directory, "target/a.txt"), "new one\nkeep\n");
    assert_eq!(read(&directory, "target/c.md"), "old three\n");
    assert!(!directory.path().join("backup").exists());
    assert!(!directory.path().join("output").exists());
}

#[test]
fn backup_and_replace_keeps_original() {
    let directory = workspace("backup-and-replace", &[("rename.kdl", rename("lines", "Line"))]);

    run(&directory).unwrap();

    assert_eq!(read(&directory, "target/a.txt"), "new one\nkeep\n");
    assert_eq!(read(&directory, "backup/a.txt"), "old one\nkeep\n");
    assert!(!directory.path().join("backup/nested/b.txt").exists());
}

#[test]
fn failing_patch_writes_nothing() {
    let directory = workspace("replace", &[("rename.kdl", rename("lines", "Missing"))]);

    let error = run(&directory).unwrap_err();

    assert!(error.to_string().contains("nothing was written"));
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
}

#[test]
fn unregistered_alias_fails() {
    let directory = workspace("replace", &[("rename.kdl", rename("cobol", "Line"))]);

    let error = run(&directory).unwrap_err();
    let related: Vec<String> = error.related().unwrap().map(|error| format!("{error:?}")).collect();

    assert_eq!(related.len(), 2);
    assert!(related[0].contains("'cobol'"));
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
}

#[test]
fn invalid_patches_stop_the_run() {
    let duplicate = patch("lines", "Line", r"old (\w+)", "new $[0]").replace("actions {", r#"actions {
    builders {
        builder "word" {
            location "Line"
            match r"(\w+)"
            build "$[0]"
        }
        builder "word" {
            location "Line"
            match r"(\w+)"
            build "$[0]"
        }
    }"#);
    let directory = workspace("replace", &[("duplicate.kdl", duplicate), ("rename.kdl", rename("lines", "Line"))]);

    let error = run(&directory).unwrap_err();
    let related: Vec<String> = error.related().unwrap().map(|error| error.to_string()).collect();

    assert!(error.to_string().contains("nothing was run"));
    assert_eq!(related.len(), 1);
    assert!(related[0].contains("Patch duplicate.kdl: word 'name': Action 'word' is already defined"));
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
}

#[test]
fn warnings_do_not_stop_the_run() {
    let unused = rename("lines", "Line").replace("actions {", r#"actions {
    builders {
        builder "unused" {
            location "Line"
            match r"(\w+)"
            build "$[0]"
        }
    }"#);
    let directory = workspace("replace", &[("rename.kdl", unused)]);

    run(&directory).unwrap();

    assert_eq!(read(&directory, "target/a.txt"), "new one\nkeep\n");
}
