// Standard Uses
use std::fs;

// Crate Uses
use crate::{project, BROKEN};
use crate::preview::operation;

// External Uses
use codeshaper_lsp::diagnostics::{coverage, diagnostics, file};
use codeshaper_lsp::trace::Traces;
use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};


pub const UNCOVERED: &str = r##"enabled false
alias ""
file "*.cpp"

actions {
    maker "unused_here" {
        make "x"
    }
    switch "kind" {
        cases old="legacy" gone="removed"
        index "@[0]"
    }
    replacer {
        location "class_specifier"
        from r"x"
        to "y"
    }
    replacer {
        location "function_definition"
        from r"nothing"
        to "#[unused_here]"
    }
    replacer {
        location "function_definition"
        from r"void (old)"
        to "#[kind($[0])]"
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
    assert_eq!(found[1].range, Range::new(Position::new(7, 19), Position::new(7, 20)));
    assert_eq!(found[1].severity, Some(DiagnosticSeverity::ERROR));
    assert!(found[1].message.starts_with("replacer #1 › from: "));
}

#[test]
fn parse_errors_use_the_label() {
    let text = "enabled \"yes\"\nalias \"\"\nfile \"*.cpp\"\nactions {\n}\n";
    let (_directory, project) = project(&[("broken.kdl", text)]);

    let found = diagnostics(&codeshaper::registry(), &project.join("patches/broken.kdl"), text);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].range, Range::new(Position::new(0, 8), Position::new(0, 13)));
}

#[test]
fn settings_errors_are_reported() {
    let (_directory, project) = project(&[]);
    let text = "name \"Test\"\ntarget \"text\"\n";
    fs::write(project.join("project.kdl"), text).unwrap();

    let found = diagnostics(&codeshaper::registry(), &project.join("project.kdl"), text);

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
    assert_eq!(found[1].range, Range::new(Position::new(9, 32), Position::new(9, 41)));
}

#[test]
fn coverage_tells_locations_from_references() {
    let (_directory, project) = operation();
    let reported = |location: &str, reference: &str, more: &str| {
        let fields = format!("location \"{location}\"\nreference-location \"{reference}\"\n");
        let text = format!(
            "enabled true\nfile \"*.cpp\"\nactions {{\nbuilder \"b\" {{\n{fields}match r\"(\\w+)\"\nbuild \"$[0]\"\n{more}\n}}\n\
            replacer {{\nlocation \"function_definition\"\nfrom \"old\"\nto \"#[b]\"\n}}\n\
            replacer \"r\" {{\n{fields}from r\"\\w+\"\nto \"x\"\n}}\n}}\n"
        );
        let trace = Traces::default().get(&codeshaper::registry(), &project, &text).unwrap();

        coverage(&text, &trace).into_iter()
            .filter(|found| found.code == Some(NumberOrString::String("location-unmatched".to_string())))
            .map(|found| (found.message, text.lines().nth(found.range.start.line as usize).unwrap().to_string()))
            .collect::<Vec<_>>()
    };
    let location = |path: &str| (
        format!("{path} › location: matches nothing in 2 target file(s)"), "location \"class_specifier\"".to_string()
    );
    let reference = |path: &str| (
        format!("{path} › reference-location: never found from the places of `location` in 2 target file(s)"),
        "reference-location \"class_specifier\"".to_string()
    );

    for more in ["", "collect true"] {
        assert_eq!(reported("function_definition", "identifier", more), vec![]);
        assert_eq!(reported("function_definition", "class_specifier", more), vec![reference("b"), reference("r")]);
        assert_eq!(reported("class_specifier", "identifier", more), vec![location("b"), location("r")]);
    }
}

#[test]
fn unmatched_files_are_reported_once() {
    let (_directory, project) = operation();
    let text = UNCOVERED.replace("enabled false", "enabled true").replace("*.cpp", "*.missing");
    let trace = Traces::default().get(&codeshaper::registry(), &project, &text).unwrap();

    let found = file(&codeshaper::registry(), &project, &text);
    assert_eq!(codes(&found), vec!["file-not-matched"]);
    assert_eq!(found[0].range, Range::new(Position::new(2, 5), Position::new(2, 16)));
    assert_eq!(coverage(&text, &trace), vec![]);

    assert_eq!(file(&codeshaper::registry(), &project, UNCOVERED), vec![]);
}

fn hints(kind: &str, fields: &str) -> Vec<(String, String)> {
    let (_directory, project) = project(&[]);
    let text = format!("enabled true\nalias \"\"\nfile \"*.cpp\"\nactions {{\n{kind} {fields}\n}}\n");

    let found = diagnostics(&codeshaper::registry(), &project.join("patches/hints.kdl"), &text);

    found.iter().zip(codes(&found))
        .filter(|(diagnostic, _)| diagnostic.severity == Some(DiagnosticSeverity::HINT))
        .map(|(diagnostic, code)| {
            let line = text.lines().nth(diagnostic.range.start.line as usize).unwrap();
            (code, line[diagnostic.range.start.character as usize..diagnostic.range.end.character as usize].to_string())
        })
        .collect()
}

#[test]
fn unused_groups_are_hinted() {
    let found = hints("replacer", r#"{
location "function_definition"
from r"(a)(?P<n>b)((?:c)d)"
to "$[n] $[0]"
}"#);

    assert_eq!(found, vec![("unused-group".to_string(), "((?:c)d)".to_string())]);
}

#[test]
fn patterns_matching_nothing_are_hinted() {
    let replacer = |from: &str| hints("replacer", &format!("{{\nlocation \"function_definition\"\nfrom r\"{from}\"\nto \"x\"\n}}"));
    let builder = |regex: &str| hints("builder", &format!("\"b\" {{\nlocation \"function_definition\"\nmatch r\"{regex}\"\nbuild \"x\"\n}}"));

    assert_eq!(replacer("a*"), vec![("matches-empty".to_string(), "r\"a*\"".to_string())]);
    assert_eq!(replacer(""), vec![("matches-empty".to_string(), "r\"\"".to_string())]);
    assert_eq!((replacer("a+"), replacer("^"), builder("")), (vec![], vec![], vec![]));
    assert_eq!(builder("a?"), vec![("matches-empty".to_string(), "r\"a?\"".to_string())]);
}

#[test]
fn diagnostics_use_names() {
    let (_directory, project) = operation();
    let named = UNCOVERED.replacen("    replacer {", "    replacer \"classes\" {", 1);
    let trace = Traces::default().get(&codeshaper::registry(), &project, &named).unwrap();
    let messages = coverage(&named, &trace).into_iter().map(|found| found.message).collect::<Vec<_>>();

    assert_eq!(messages[2..], ["classes › location: matches nothing in 2 target file(s)", "replacer #2 › from: never matches in the 2 located content(s)"]);

    let (_directory, project) = crate::project(&[]);
    let wrong = BROKEN.replacen("    replacer {", "    replacer \"a/b\" {", 1);
    let found = diagnostics(&codeshaper::registry(), &project.join("patches/broken.kdl"), &wrong);
    assert_eq!(codes(&found)[..2], ["incorrect-name", "location-not-found"]);
    assert!(found[0].message.starts_with("replacer #1 › name: A replacer's name can't have"), "{}", found[0].message);

    let line = found[0].range.start.line as usize;
    assert_eq!(&wrong.lines().nth(line).unwrap()[found[0].range.start.character as usize..found[0].range.end.character as usize], "\"a/b\"");
}
