// Standard Uses
use std::fs;
use std::ops::Range;

// Crate Uses
use crate::operation::{patch, read, registry, workspace};

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::registry::Registry;
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::validation::DiagnosticKind;
use miette::{bail, Result};
use tempfile::TempDir;


struct StrictBackend(&'static str, &'static str);

impl LanguageBackend for StrictBackend {
    fn name(&self) -> &str { self.0 }

    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>> {
        self.check_location(location)?;

        Ok(source.match_indices("old").map(|(start, word)| start..start + word.len()).collect())
    }

    fn check_location(&self, location: &str) -> Result<()> {
        if location != self.1 { bail!("Unknown location {location}") }

        Ok(())
    }
}

fn both() -> Registry {
    let mut registry = registry();
    registry.add_backend(Box::new(StrictBackend("words", "Word")));
    registry.add_backend(Box::new(StrictBackend("numbers", "Number")));

    registry
}

fn project(ast_set: &str, patches: &[(&str, String)]) -> TempDir {
    let directory = workspace("replace", patches);
    let settings = directory.path().join("shaping/settings.kdl");
    fs::write(&settings, fs::read_to_string(&settings).unwrap().replace("ast-set \"lines\"", ast_set)).unwrap();

    directory
}

fn word(alias: &str, file: &str) -> String {
    patch(alias, "Word", "old", "new").replace("**/*.txt", file)
}

fn checked(directory: &TempDir) -> Result<Vec<(DiagnosticKind, String)>> {
    let operation = Operation::from_file(&directory.path().join("configuration.kdl"))?;

    Ok(operation.check(&both())?.into_iter().map(|(_, diagnostic)| (diagnostic.kind, diagnostic.message)).collect())
}

fn ran(directory: &TempDir) -> Result<()> {
    Operation::from_file(&directory.path().join("configuration.kdl"))?.run(&both()).map(|_| ())
}


#[test]
fn files_of_one_extension_can_differ() {
    let directory = project("ast-set { lines \"*.txt\"; words \"nested/*.txt\" \"*.md\"; }", &[("word.kdl", word("", "c.md"))]);

    ran(&directory).unwrap();
    assert_eq!(read(&directory, "target/c.md"), "new three\n");

    let directory = project("ast-set { lines \"*.txt\"; words \"a.txt\"; }", &[("word.kdl", word("", "a.txt"))]);
    ran(&directory).unwrap();
    assert_eq!(read(&directory, "target/a.txt"), "new one\nkeep\n");
}

#[test]
fn alias_wins_over_every_form() {
    for ast_set in ["ast-set \"lines\"", "ast-set \"target\"", "ast-set { lines \"*\"; }"] {
        let directory = project(ast_set, &[("word.kdl", word("words", "a.txt"))]);

        assert_eq!(checked(&directory).unwrap(), vec![], "{ast_set}");
        ran(&directory).unwrap();
        assert_eq!(read(&directory, "target/a.txt"), "new one\nkeep\n", "{ast_set}");
    }
}

#[test]
fn missing_languages_are_refused_before_writing() {
    for (ast_set, alias, message) in [
        ("ast-set \"target\"", "", "'a.txt' has no language, the target gives none"),
        ("ast-set { words \"*.md\"; }", "", "'a.txt' has no language, no line of 'ast-set' matches"),
        ("ast-set \"lines\"", "gone", "'gone' is not a known language, known languages are: lines, numbers, words")
    ] {
        let directory = project(ast_set, &[("other.kdl", word("words", "c.md")), ("word.kdl", word(alias, "a.txt"))]);

        assert_eq!(checked(&directory).unwrap(), vec![(DiagnosticKind::LanguageNotFound, message.to_string())], "{ast_set}");
        assert!(ran(&directory).is_err(), "{ast_set}");
        assert_eq!(read(&directory, "target/c.md"), "old three\n", "{ast_set}");
    }
}

#[test]
fn unknown_languages_in_the_set_are_errors() {
    for ast_set in ["ast-set \"gone\"", "ast-set { words \"*\"; gone \"x\"; }"] {
        let directory = project(ast_set, &[("word.kdl", word("words", "a.txt"))]);

        assert!(checked(&directory).unwrap_err().to_string().contains("'gone', which is not a known language"), "{ast_set}");
        assert!(ran(&directory).is_err(), "{ast_set}");
    }
}

#[test]
fn locations_must_fit_every_matched_language() {
    let directory = project("ast-set { numbers \"nested/*\"; words \"a.txt\"; }", &[("word.kdl", word("", "**/*.txt"))]);
    assert_eq!(checked(&directory).unwrap(), vec![(DiagnosticKind::LocationNotFound, "numbers: Unknown location Word".to_string())]);
    assert!(ran(&directory).is_err());

    let directory = project("ast-set { words \"*.txt\"; }", &[("word.kdl", patch("", "Line", "old", "new"))]);
    assert_eq!(checked(&directory).unwrap(), vec![(DiagnosticKind::LocationNotFound, "Unknown location Line".to_string())]);
    assert!(ran(&directory).is_err());
}

#[test]
fn patches_matching_nothing_have_no_language() {
    let directory = project("ast-set \"target\"", &[("word.kdl", word("", "*.rs"))]);
    let kinds: Vec<DiagnosticKind> = checked(&directory).unwrap().into_iter().map(|(kind, _)| kind).collect();

    assert_eq!(kinds, vec![DiagnosticKind::FileNotMatched]);
    ran(&directory).unwrap();
}
