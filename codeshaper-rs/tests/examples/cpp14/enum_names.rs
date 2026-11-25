// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses
use crate::examples::{self, copy_directory, read, result_with};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;


const EXAMPLE: &str = "examples/CPP14/Enum Names/";


#[test]
fn runs_end_to_end() {
    examples::runs_to_the_expected_result(EXAMPLE, &[
        ("Color.h", &["enum_names.kdl"]), ("Shape.h", &["enum_names.kdl"])
    ], &["Figure.cpp"]);
}

#[test]
fn preview_matches_the_run() {
    examples::preview_matches_the_run(EXAMPLE);
}

#[test]
fn patches_validate() {
    let operation = Operation::from_file(&Path::new(EXAMPLE).join("configuration.kdl")).unwrap();

    assert_eq!(operation.check(&registry()).unwrap(), vec![]);
}

#[test]
fn variables_change_the_result() {
    let expected = read(&Path::new(EXAMPLE).join("expected_result/Shape.h"));

    assert_eq!(result_with(EXAMPLE, &[], "Shape.h"), expected);
    assert_eq!(result_with(EXAMPLE, &[("suffix", "_names")], "Shape.h"), expected.replace("Names[]", "_names[]"));
    assert_eq!(result_with(EXAMPLE, &[("type", "std::string_view")], "Shape.h"), expected.replace("const char*", "std::string_view"));
}

#[test]
fn only_variables_can_be_set() {
    let mut operation = Operation::from_file(&Path::new(EXAMPLE).join("configuration.kdl")).unwrap();

    assert_eq!(operation.set("note", "x").unwrap_err().to_string(), "'note' is a constant, constants can't be set");
    assert!(operation.set("other", "x").is_err());
}

#[test]
fn nothing_is_written_without_a_value() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_directory(Path::new(EXAMPLE), root);

    let path = root.join("configuration.kdl");
    let configuration = read(&path);
    fs::write(&path, configuration.replace("variables type=\"const char*\"", "")).unwrap();

    let error = Operation::from_file(&path).unwrap().run(&registry()).unwrap_err().to_string();
    assert!(error.starts_with("Variable 'type' has no value"), "{error}");
    assert_eq!(read(&root.join("target/Color.h")), read(&Path::new(EXAMPLE).join("target/Color.h")));
    assert!(!root.join("target_backup").exists());
}
