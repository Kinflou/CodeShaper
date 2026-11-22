// Standard Uses
use std::fs;

// Crate Uses
use crate::{project, BROKEN};

// External Uses
use codeshaper_lsp::diagnostics::diagnostics;
use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};


fn codes(found: &[lsp_types::Diagnostic]) -> Vec<String> {
    found.iter()
        .map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) => code.clone(),
            _ => String::new()
        })
        .collect()
}


#[test]
fn patch_diagnostics_point_at_values() {
    let (_directory, project) = project(&[("broken.kdl", BROKEN)]);
    let path = project.join("patches/broken.kdl");

    let found = diagnostics(&codeshaper::registry(), &path, BROKEN);

    assert_eq!(codes(&found), vec!["location-not-found", "incorrect-regex", "action-not-found"]);
    assert_eq!(found[1].range, Range::new(Position::new(8, 17), Position::new(8, 25)));
    assert_eq!(found[1].severity, Some(DiagnosticSeverity::ERROR));
    assert!(found[1].message.starts_with("replacer #1 › from: "));
}

#[test]
fn parse_errors_use_the_label() {
    let text = "enabled \"yes\"\nalias \"cpp\"\nfile \"*.cpp\"\nactions {\n}\n";
    let (_directory, project) = project(&[("broken.kdl", text)]);

    let found = diagnostics(&codeshaper::registry(), &project.join("patches/broken.kdl"), text);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].range, Range::new(Position::new(0, 8), Position::new(0, 13)));
}

#[test]
fn settings_errors_are_reported() {
    let (_directory, project) = project(&[]);
    let text = "name \"Test\"\ntarget \"text\"\n";
    fs::write(project.join("settings.kdl"), text).unwrap();

    let found = diagnostics(&codeshaper::registry(), &project.join("settings.kdl"), text);

    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("ast-set"));
}

#[test]
fn files_outside_a_project_are_ignored() {
    let directory = tempfile::tempdir().unwrap();

    let found = diagnostics(&codeshaper::registry(), &directory.path().join("notes.kdl"), BROKEN);

    assert_eq!(found, vec![]);
}
