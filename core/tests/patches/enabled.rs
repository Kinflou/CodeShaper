// Standard Uses

// Crate Uses
use crate::operation::{read, registry, rename, run, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::operation::report::Outcome;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::validation::DiagnosticKind;


const FIELDS: &str = "file \"*\"\nactions {\n}\n";

fn enabled(text: &str) -> Vec<bool> {
    Patch::parse_all("", text).unwrap().into_iter().map(|named| named.patch.enabled).collect()
}


#[test]
fn patches_are_enabled_unless_they_say_otherwise() {
    assert_eq!(enabled(FIELDS), vec![true]);
    assert_eq!(enabled(&format!("enabled true\n{FIELDS}")), vec![true]);
    assert_eq!(enabled(&format!("{FIELDS}enabled false\n")), vec![false]);
    assert_eq!(enabled(&format!("/-enabled false\n{FIELDS}")), vec![true]);
    assert_eq!(enabled(&format!("patch \"a\" {{\n{FIELDS}}}\npatch \"b\" {{\nenabled false\n{FIELDS}}}\n")), vec![true, false]);

    for wrong in ["enabled\n", "enabled 1\n", "enabled \"true\"\n", "enabled true false\n", "enabled true\nenabled true\n"] {
        assert!(Patch::parse_all("", &format!("{wrong}{FIELDS}")).is_err(), "{wrong}");
    }
}

#[test]
fn patches_made_in_code_are_enabled() {
    assert_eq!(Patch::from_kdl("", "file \"\"\nactions\n").unwrap(), Patch::default());
}

#[test]
fn a_missing_field_runs_as_true() {
    let written = rename("lines", "Line");
    let missing = written.replace("enabled true\n", "");
    let disabled = written.replace("true", "false");
    assert!(!missing.contains("enabled"));

    let results = [written, missing, disabled].map(|patch| {
        let directory = workspace("replace", &[("rename.kdl", patch)]);
        let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
        assert_eq!(operation.check(&registry()).unwrap().len(), 0);

        let patched = run(&directory).unwrap().into_iter().filter(|file| file.outcome != Outcome::Unchanged).count();
        (read(&directory, "target/a.txt"), patched)
    });

    assert_eq!(results[0], ("new one\nkeep\n".to_string(), 1));
    assert_eq!(results[1], results[0]);
    assert_eq!(results[2], ("old one\nkeep\n".to_string(), 0));
}

#[test]
fn a_missing_field_is_checked_as_true() {
    let unmatched = rename("lines", "Line").replace("**/*.txt", "*.missing");
    let found = |patch: &str| {
        let directory = workspace("replace", &[("rename.kdl", patch.to_string())]);
        let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

        operation.check(&registry()).unwrap().into_iter().map(|(_, diagnostic)| diagnostic.kind).collect::<Vec<_>>()
    };

    assert_eq!(found(&unmatched.replace("enabled true\n", "")), vec![DiagnosticKind::FileNotMatched]);
    assert_eq!(found(&unmatched.replace("enabled true", "enabled false")), vec![]);
}
