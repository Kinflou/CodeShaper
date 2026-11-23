// Standard Uses
use std::path::Path;

// Crate Uses
use crate::operation::{patch, read, registry, rename, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::operation::session::FileState;
use tempfile::TempDir;


fn two_patches(result: &str, second_location: &str) -> TempDir {
    workspace(result, &[
        ("1_rename.kdl", rename("", "Line")),
        ("2_keep.kdl", patch("", second_location, "(keep)", "kept")),
    ])
}

fn operation(directory: &TempDir) -> Operation {
    Operation::from_file(&directory.path().join("configuration.kdl")).unwrap()
}

fn states(files: Vec<(&Path, FileState)>) -> Vec<FileState> {
    files.into_iter().map(|(_, state)| state).collect()
}


#[test]
fn steps_follow_files_then_patches() {
    let directory = two_patches("replace", "Line");
    let (operation, registry) = (operation(&directory), registry());
    let mut session = operation.session(&registry).unwrap();

    assert_eq!(session.len(), 4);
    assert_eq!(states(session.files()), vec![FileState::Waiting, FileState::Waiting, FileState::Untouched]);

    let mut steps = vec![];
    while let Some(change) = session.next() {
        steps.push((change.file.display().to_string(), change.patch.clone()));
    }

    assert_eq!(steps, vec![
        ("a.txt".to_string(), "1_rename.kdl".to_string()),
        ("a.txt".to_string(), "2_keep.kdl".to_string()),
        ("nested/b.txt".to_string(), "1_rename.kdl".to_string()),
        ("nested/b.txt".to_string(), "2_keep.kdl".to_string()),
    ]);
    assert_eq!(session.position(), 4);
}

#[test]
fn next_and_previous_move_through_history() {
    let directory = two_patches("replace", "Line");
    let (operation, registry) = (operation(&directory), registry());
    let mut session = operation.session(&registry).unwrap();

    session.next();
    assert_eq!(session.content(0), Some("new one\nkeep\n"));
    assert_eq!(states(session.files())[..2], [FileState::Processing, FileState::Waiting]);

    session.next();
    assert_eq!(session.content(0), Some("new one\nkept\n"));
    assert_eq!(states(session.files())[..2], [FileState::Processed, FileState::Waiting]);

    let undone = session.previous().unwrap();
    assert_eq!(undone.patch, "2_keep.kdl");
    assert_eq!(session.content(0), Some("new one\nkeep\n"));
    assert_eq!(session.state(0), FileState::Processing);

    session.previous();
    assert_eq!(session.content(0), Some("old one\nkeep\n"));
    assert_eq!(session.state(0), FileState::Waiting);
    assert!(session.previous().is_none());
    assert!(session.applied().is_empty());

    let redone = session.next().unwrap();
    assert_eq!(redone.result.as_deref().unwrap(), "new one\nkeep\n");
    session.next();
    assert_eq!(session.content(0), Some("new one\nkept\n"));
    assert_eq!(session.applied().len(), 2);
}

#[test]
fn nothing_written_until_finish() {
    let directory = two_patches("replace", "Line");
    let (operation, registry) = (operation(&directory), registry());
    let mut session = operation.session(&registry).unwrap();

    while session.next().is_some() {}
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");

    session.finish().unwrap();
    assert_eq!(read(&directory, "target/a.txt"), "new one\nkept\n");
    assert_eq!(read(&directory, "target/nested/b.txt"), "kept\n");
}

#[test]
fn failed_step_marks_file_until_undone() {
    let directory = two_patches("replace", "Missing");
    let (operation, registry) = (operation(&directory), registry());
    let mut session = operation.session(&registry).unwrap();

    session.next();
    let failed = session.next().unwrap();
    assert!(failed.result.is_err());
    assert_eq!(session.content(0), Some("new one\nkeep\n"));
    assert_eq!(session.state(0), FileState::Error);

    session.previous();
    assert_eq!(session.state(0), FileState::Processing);

    let error = session.finish().unwrap_err();
    assert!(error.to_string().contains("nothing was written"));
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
}

#[test]
fn stop_keeps_history_and_time() {
    let directory = two_patches("replace", "Line");
    let (operation, registry) = (operation(&directory), registry());
    let mut session = operation.session(&registry).unwrap();

    assert!(!session.is_running());
    session.start();
    assert!(session.is_running());
    session.next();
    session.stop();

    let elapsed = session.elapsed();
    assert!(!session.is_running());
    assert!(elapsed > std::time::Duration::ZERO);
    assert_eq!(session.elapsed(), elapsed);
    assert_eq!(session.position(), 1);

    session.start();
    session.next();
    assert!(session.elapsed() >= elapsed);
    assert_eq!(session.position(), 2);
}

#[test]
fn previews_hold_the_final_contents() {
    let directory = two_patches("replace", "Line");
    let (operation, registry) = (operation(&directory), registry());

    let preview = operation.session(&registry).unwrap().preview().unwrap();
    let files: Vec<(&Path, &str, &str)> = preview.files.iter()
        .map(|file| (file.path.as_path(), file.before.as_str(), file.result.as_str()))
        .collect();

    assert_eq!(files, vec![
        (Path::new("a.txt"), "old one\nkeep\n", "new one\nkept\n"),
        (Path::new("nested/b.txt"), "keep\n", "kept\n"),
    ]);
    assert!(preview.errors.is_empty());
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
}

#[test]
fn previews_keep_failures_beside_the_files() {
    let directory = two_patches("replace", "Missing");
    let (operation, registry) = (operation(&directory), registry());

    let preview = operation.session(&registry).unwrap().preview().unwrap();

    assert_eq!(preview.files.iter().map(|file| file.result.as_str()).collect::<Vec<_>>(), vec!["new one\nkeep\n"]);
    assert_eq!(preview.errors.iter().map(|(path, _)| path.as_path()).collect::<Vec<_>>(), vec![Path::new("a.txt"), Path::new("nested/b.txt")]);
}

#[test]
fn previews_can_be_stopped() {
    let directory = two_patches("replace", "Line");
    let (operation, registry) = (operation(&directory), registry());

    assert!(operation.session(&registry).unwrap().preview_while(&|| false).is_err());
}
