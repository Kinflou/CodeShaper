// Standard Uses
use std::fs;
use std::path::PathBuf;

// Crate Uses
use crate::preview::{operation, RENAME};

// External Uses
use codeshaper_lsp::completion::{completion, settings as suggested};
use codeshaper_lsp::diagnostics::{diagnostics, file, redundant_alias};
use codeshaper_lsp::hover::settings as explained;
use codeshaper_lsp::target::{backends, forget_languages};
use codeshaper_lsp::syntax::Outline;
use lsp_types::{CompletionItem, Diagnostic, DiagnosticSeverity, NumberOrString};
use tempfile::TempDir;


fn settings(ast_set: &str) -> String {
    format!("name \"A\"\ntarget \"text\"\n{ast_set}\ndescription \"\"\n")
}

fn project(ast_set: &str) -> (TempDir, PathBuf) {
    let (directory, project) = operation();
    fs::write(project.join("settings.kdl"), settings(ast_set)).unwrap();

    (directory, project)
}

fn labels(items: &[CompletionItem]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics.iter()
        .filter_map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) => Some(code.clone()),
            _ => None
        })
        .collect()
}


#[test]
fn languages_come_from_the_matched_files() {
    let registry = codeshaper::registry();
    let (_directory, project) = project("ast-set {\n    gdscript \"a.cpp\"\n    cpp \"b.cpp\"\n}");
    let names = |file: &str| {
        backends(&registry, &project, &Outline::parse(&format!("file \"{file}\"\n"))).iter().map(|backend| backend.name().to_string()).collect::<Vec<_>>()
    };

    assert_eq!(names("a.cpp"), vec!["gdscript"]);
    assert_eq!(names("b.cpp"), vec!["cpp"]);
    assert_eq!(names("*.cpp"), vec!["gdscript", "cpp"]);
    assert_eq!(names("*.rs"), Vec::<String>::new());

    fs::write(project.join("settings.kdl"), settings("ast-set {\n    cpp \"*\"\n}")).unwrap();
    forget_languages();
    assert_eq!(names("a.cpp"), vec!["cpp"]);
}

#[test]
fn alias_and_ast_set_suggest_languages() {
    let registry = codeshaper::registry();
    let (_directory, project) = project("ast-set \"cpp\"");

    let text = "alias \"\"\n";
    assert_eq!(labels(&completion(&registry, &project, text, 7)), registry.languages());

    let one = settings("ast-set \"\"");
    let found = suggested(&registry, &one, one.find("ast-set").unwrap() + 9);
    assert_eq!(labels(&found).last(), Some(&"target"));
    assert!(labels(&found).contains(&"cpp"));

    let block = settings("ast-set {\n    \n}");
    assert_eq!(labels(&suggested(&registry, &block, block.find("    ").unwrap() + 4)), registry.languages());
    assert_eq!(labels(&suggested(&registry, &block, 6)), Vec::<&str>::new());
}

#[test]
fn unknown_languages_are_pointed_at() {
    let registry = codeshaper::registry();
    let (_directory, project) = project("ast-set \"cpp\"");
    let found = |ast_set: &str| diagnostics(&registry, &project.join("settings.kdl"), &settings(ast_set));

    assert_eq!(found("ast-set \"cpp\""), vec![]);
    assert_eq!(found("ast-set \"target\""), vec![]);
    assert_eq!(found("ast-set \"\"").len(), 1);

    let unknown = found("ast-set {\n    cpp \"*.cpp\"\n    cobol \"*.cob\"\n}");
    assert_eq!(codes(&unknown), vec!["language-not-found"]);
    assert_eq!((unknown[0].range.start.line, unknown[0].range.start.character, unknown[0].range.end.character), (4, 4, 9));
    assert!(unknown[0].message.contains("'cobol' is not a known language"));
}

#[test]
fn files_without_a_language_are_reported() {
    let registry = codeshaper::registry();
    let (_directory, project) = project("ast-set {\n    cpp \"a.cpp\"\n}");

    let found = file(&registry, &project, RENAME);
    assert_eq!(codes(&found), vec!["language-not-found"]);
    assert_eq!(found[0].severity, Some(DiagnosticSeverity::ERROR));
    assert!(found[0].message.contains("'b.cpp' has no language, no line of 'ast-set' matches"), "{}", found[0].message);

    let found = file(&registry, &project, &RENAME.replace("alias \"\"", "alias \"cobol\""));
    assert!(found[0].message.contains("patch › alias: 'cobol' is not a known language"), "{}", found[0].message);
}

#[test]
fn redundant_aliases_are_faded() {
    let registry = codeshaper::registry();
    let (_directory, project) = project("ast-set \"cpp\"");
    let path = project.join("patches/rename.kdl");
    let named = RENAME.replace("alias \"\"", "alias \"cpp\"");

    assert!(codes(&diagnostics(&registry, &path, &named)).contains(&"redundant-alias".to_string()));
    assert_eq!(redundant_alias(&project, &named).map(|range| &named[range]), Some("alias \"cpp\""));
    assert_eq!(redundant_alias(&project, RENAME), None);
    assert_eq!(redundant_alias(&project, &named.replace("\"cpp\"", "\"gdscript\"")), None);

    fs::write(project.join("settings.kdl"), settings("ast-set {\n    cpp \"*\"\n}")).unwrap();
    assert_eq!(redundant_alias(&project, &named), None);
}

#[test]
fn ast_set_explains_what_each_language_gets() {
    let registry = codeshaper::registry();
    let (_directory, project) = project("ast-set \"cpp\"");
    let hovered = |ast_set: &str, needle: &str| {
        let text = settings(ast_set);
        explained(&registry, &project, &text, text.find(needle).unwrap() + 1).map(|(value, _)| value)
    };

    let value = hovered("ast-set \"cpp\"", "ast-set").unwrap();
    assert!(value.starts_with("Every file is `cpp`\n\n- `cpp`: 2 file(s)\n\nKnown languages are `cpp`"), "{value}");
    let value = hovered("ast-set \"target\"", "ast-set").unwrap();
    assert!(value.starts_with("The target names each file's language\n\n- 2 file(s) without a language"), "{value}");
    let value = hovered("ast-set {\n    gdscript \"a.*\"\n}", "a.*").unwrap();
    assert!(value.contains("- 1 file(s) without a language\n- `gdscript`: 1 file(s)"), "{value}");
    assert_eq!(hovered("ast-set \"cpp\"", "target \""), None);
}
