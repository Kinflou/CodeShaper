// Standard Uses
use std::fs;

// Crate Uses
use crate::operation::{patch, read, report, run, workspace};

// External Uses
use codeshaper_core::shaping::lines::{restore, unix};
use codeshaper_core::shaping::operation::report::Outcome;
use codeshaper_core::shaping::patch::Patch;


#[test]
fn windows_files_are_matched_and_keep_their_endings() {
    let directory = workspace("replace", &[("join.kdl", patch("", "File", r"old (\w+)\nkeep", "new $[0]\nkept"))]);
    fs::write(directory.path().join("target/a.txt"), "old one\r\nkeep\r\n").unwrap();
    fs::write(directory.path().join("target/nested/b.txt"), "keep\r\n").unwrap();

    assert_eq!(run(&directory).unwrap()[..2], [
        report("a.txt", Outcome::Patched(vec!["join.kdl".to_string()])),
        report("nested/b.txt", Outcome::Unchanged)
    ]);
    assert_eq!(read(&directory, "target/a.txt"), "new one\r\nkept\r\n");
    assert_eq!(read(&directory, "target/nested/b.txt"), "keep\r\n");
}

#[test]
fn files_without_a_match_keep_mixed_endings() {
    let directory = workspace("replace", &[("join.kdl", patch("", "File", "missing", "found"))]);
    fs::write(directory.path().join("target/a.txt"), "one\r\ntwo\nthree\n").unwrap();

    run(&directory).unwrap();

    assert_eq!(read(&directory, "target/a.txt"), "one\r\ntwo\nthree\n");
}

#[test]
fn windows_patches_match_and_write_like_unix_ones() {
    let content = "enabled true\nalias \"\"\nfile \"a.txt\"\nactions {\n    replacers {\n        replacer {\n            location \"\"\n            from \"a\nb\"\n            to r#\"c\nd\"#\n        }\n    }\n}\n";

    assert_eq!(Patch::from_kdl("", &content.replace('\n', "\r\n")).unwrap(), Patch::from_kdl("", content).unwrap());
}

#[test]
fn the_most_common_ending_is_restored() {
    assert_eq!(restore("a\r\nb\r\nc\n", "x\ny\r\n"), "x\r\ny\r\n");
    assert_eq!(restore("a\r\nb\nc\n", "x\ny\r\n"), "x\ny\n");
    assert_eq!(unix("a\r\n\rb\n"), "a\n\rb\n");
}
