// Standard Uses
use std::{fs, slice};

// Crate Uses
use crate::{project, BROKEN};
use crate::preview::operation;

// External Uses
use codeshaper_lsp::diagnostics::{coverage, diagnostics, file};
use codeshaper_lsp::trace::Traces;
use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};


pub const UNCOVERED: &str = r##"enabled false
alias "cpp"
file "*.cpp"

actions {
    makers {
        maker "unused_here" {
            make "x"
        }
    }
    switchers {
        switch "kind" {
            cases old="legacy" gone="removed"
            index "@[0]"
        }
    }
    replacers {
        replacer {
            location "ClassSpecifier"
            from r"x"
            to "y"
        }
        replacer {
            location "FunctionDefinition"
            from r"nothing"
            to "#[unused_here]"
        }
        replacer {
            location "FunctionDefinition"
            from r"void (old)"
            to "#[kind]($[0])"
        }
    }
}
"##;


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
    assert_eq!(found[1].range, Range::new(Position::new(8, 23), Position::new(8, 24)));
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

#[test]
fn coverage_reports_what_matches_nothing() {
    let (_directory, project) = operation();
    let trace = Traces::default().get(&codeshaper::registry(), &project, UNCOVERED).unwrap();

    let found = coverage(UNCOVERED, &trace);
    let summary: Vec<(String, Option<DiagnosticSeverity>, &str)> = codes(&found).into_iter().zip(&found)
        .map(|(code, diagnostic)| (code, diagnostic.severity, diagnostic.message.as_str()))
        .collect();

    assert_eq!(summary, vec![
        ("never-called".to_string(), Some(DiagnosticSeverity::HINT), "unused_here: never called in 2 target file(s)"),
        ("case-unused".to_string(), Some(DiagnosticSeverity::HINT), "kind › cases: 'gone' is never used in 2 target file(s)"),
        ("location-unmatched".to_string(), Some(DiagnosticSeverity::WARNING), "replacer #1 › location: matches nothing in 2 target file(s)"),
        ("from-unmatched".to_string(), Some(DiagnosticSeverity::WARNING), "replacer #2 › from: never matches in the 2 located content(s)"),
    ]);
    assert_eq!(found[1].range, Range::new(Position::new(12, 36), Position::new(12, 45)));
}

#[test]
fn unmatched_files_are_reported_once() {
    let (_directory, project) = operation();
    let text = UNCOVERED.replace("enabled false", "enabled true").replace("*.cpp", "*.missing");
    let trace = Traces::default().get(&codeshaper::registry(), &project, &text).unwrap();

    let found = file(&codeshaper::registry(), &project, &text).unwrap();
    assert_eq!(codes(slice::from_ref(&found)), vec!["file-not-matched"]);
    assert_eq!(found.range, Range::new(Position::new(2, 5), Position::new(2, 16)));
    assert_eq!(coverage(&text, &trace), vec![]);

    assert_eq!(file(&codeshaper::registry(), &project, UNCOVERED), None);
}
