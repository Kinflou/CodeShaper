// Standard Uses
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper_lsp::completion::document;
use codeshaper_lsp::diagnostics::diagnostics;
use codeshaper_lsp::document::{kind, DocumentKind};
use codeshaper_lsp::paths::{check, check_patterns, completion, hover, open, opening, target, Opening};
use codeshaper_lsp::position::LineIndex;
use codeshaper_target_text::options::Field;
use lsp_types::{CompletionItem, CompletionTextEdit, Diagnostic, DiagnosticSeverity, NumberOrString};
use tempfile::TempDir;


fn write(directory: &TempDir, path: &str, content: &str) -> PathBuf {
    let path = directory.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();

    path
}

fn workspace() -> (TempDir, [PathBuf; 2]) {
    let directory = tempfile::tempdir().unwrap();

    for file in ["a.cpp", "b.cpp", "src/c.cpp", "src/deep/d.h", "src/é.cpp", ".hidden/x.cpp", "docs/e.md"] {
        write(&directory, &format!("code/{file}"), "");
    }

    let settings = write(&directory, "shaping/project.kdl", &settings(""));
    let configuration = write(&directory, "configuration.kdl", &configuration("code/", ""));
    (directory, [settings, configuration])
}

fn block(body: &str) -> String {
    if body.is_empty() { String::new() } else { format!(" {{\n    {body}\n}}") }
}

fn settings(body: &str) -> String {
    format!("name \"A\"\ntarget \"text\"{}\nast-set \"cpp\"\ndescription \"\"\n", block(body))
}

fn configuration(target: &str, body: &str) -> String {
    format!("name \"A\"\nproject \"shaping/\"\ntarget \"{target}\"{}\noutput \"out/\"\nbackup \"backup/\"\nresult \"replace\"\n", block(body))
}

fn document_text(path: &Path, body: &str) -> String {
    match kind(path) {
        DocumentKind::Settings => settings(body),
        DocumentKind::Configuration => configuration("code/", body),
        _ => panic!("expected settings or a configuration")
    }
}

fn labels(items: &[CompletionItem]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

fn suggested(path: &Path, marked: &str) -> (String, usize, Vec<CompletionItem>) {
    let offset = marked.find('|').unwrap();
    let text = marked.replacen('|', "", 1);
    let items = completion(path, &text, offset);

    (text, offset, items)
}

fn chosen(path: &Path, marked: &str, label: &str) -> String {
    let (text, _, items) = suggested(path, marked);
    let item = items.iter().find(|item| item.label == label).unwrap();
    let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else { panic!("expected an edit") };
    let index = LineIndex::new(&text);

    format!("{}{}{}", &text[..index.offset(edit.range.start)], edit.new_text, &text[index.offset(edit.range.end)..])
}

fn found(text: &str, diagnostics: &[Diagnostic]) -> Vec<(String, String)> {
    let index = LineIndex::new(text);

    diagnostics.iter()
        .map(|diagnostic| {
            let Some(NumberOrString::String(code)) = &diagnostic.code else { panic!("expected a code") };
            (code.clone(), text[index.offset(diagnostic.range.start)..index.offset(diagnostic.range.end)].to_string())
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected.iter().map(|(code, text)| (code.to_string(), text.to_string())).collect()
}


#[test]
fn files_are_suggested_by_segment() {
    let (_directory, paths) = workspace();

    for path in &paths {
        let at = |body: &str| labels(&suggested(path, &document_text(path, body)).2).into_iter().map(str::to_string).collect::<Vec<_>>();

        for body in ["files \"|\"", "files {\n        \"|\"\n    }"] {
            assert_eq!(at(body), vec!["**/", "*.cpp", "*.h", "*.md", "a.cpp", "b.cpp", "docs/", "src/"], "{body}");
        }
        for body in ["files \"src/|\"", "files {\n        \"src/|\"\n    }", "files {\n        \"a.cpp\"\n        \"src/|\"\n    }"] {
            assert_eq!(at(body), vec!["**/", "*.cpp", "*.h", "c.cpp", "deep/", "é.cpp"], "{body}");
        }

        assert_eq!(at("files \"src/é|\""), vec!["**/", "*.cpp", "*.h", "c.cpp", "deep/", "é.cpp"]);
        assert_eq!(at("files \".|\""), vec!["**/", "*.cpp", "*.h", "*.md", ".hidden/", "a.cpp", "b.cpp", "docs/", "src/"]);
        assert_eq!(at("files \"missing/|\""), vec!["**/"]);
        assert_eq!(at("exclude {\n        \"src/**/|\"\n    }"), vec!["*.cpp", "*.h"]);
        assert_eq!(at("files \"*/|\""), vec!["**/", "*.cpp", "*.h", "*.md"]);
    }
}

#[test]
fn listed_files_are_left_out() {
    let (_directory, paths) = workspace();

    for path in &paths {
        let at = |body: &str| labels(&suggested(path, &document_text(path, body)).2).into_iter().map(str::to_string).collect::<Vec<_>>();

        assert_eq!(at("files \"a.cpp\" \"|\""), vec!["**/", "*.cpp", "*.h", "*.md", "b.cpp", "docs/", "src/"]);
        assert_eq!(at("files {\n        \"src/c.cpp\"\n        \"src/|\"\n    }"), vec!["**/", "*.cpp", "*.h", "deep/", "é.cpp"]);
        assert_eq!(at("files \"a.cpp|\""), vec!["**/", "*.cpp", "*.h", "*.md", "a.cpp", "b.cpp", "docs/", "src/"]);
    }
}

#[test]
fn half_typed_blocks_get_suggestions() {
    let (_directory, paths) = workspace();

    for path in &paths {
        let open = document_text(path, "files {\n        \"src/|\n        \"a.cpp\"\n    }");
        assert_eq!(labels(&suggested(path, &open).2), vec!["**/", "*.cpp", "*.h", "c.cpp", "deep/", "é.cpp"]);
        assert_eq!(chosen(path, &open, "c.cpp"), open.replace('|', "c.cpp"));

        let crlf = open.replace('\n', "\r\n");
        assert_eq!(chosen(path, &crlf, "c.cpp"), crlf.replace('|', "c.cpp"));

        let bare = document_text(path, "files {\n        |\n    }");
        assert_eq!(chosen(path, &bare, "src/"), bare.replace('|', "\"src/\""));
        let started = document_text(path, "files {\n        a.c|\n    }");
        assert_eq!(chosen(path, &started, "b.cpp"), started.replace("a.c|", "\"b.cpp\""));
    }
}

#[test]
fn one_segment_is_replaced() {
    let (_directory, paths) = workspace();

    for path in &paths {
        let text = |body: &str| document_text(path, body);

        assert_eq!(chosen(path, &text("files \"sr|c/deep/d.h\""), "src/"), text("files \"src/deep/d.h\""));
        assert_eq!(chosen(path, &text("files \"src/|deep\""), "c.cpp"), text("files \"src/c.cpp\""));
        assert_eq!(chosen(path, &text("files {\n        \"|\"\n    }"), "a.cpp"), text("files {\n        \"a.cpp\"\n    }"));
    }
}

#[test]
fn only_paths_get_suggestions() {
    let (directory, paths) = workspace();
    let [settings_path, configuration_path] = &paths;

    for path in &paths {
        for text in ["name \"|\"\n", "name \"A\"\nfiles \"|\"\n", "other \"a\" {\n    files \"|\"\n}\n"] {
            assert_eq!(labels(&suggested(path, text).2), Vec::<&str>::new(), "{text}");
        }
        assert_eq!(labels(&suggested(path, &document_text(path, "|")).2), Vec::<&str>::new());
    }

    for name in ["project", "target", "output", "backup"] {
        assert_eq!(labels(&suggested(configuration_path, &format!("{name} \"|\"\n")).2), vec!["code/", "shaping/"]);
        assert_eq!(labels(&suggested(configuration_path, &format!("{name} \"code/|\"\n")).2), vec!["docs/", "src/"]);
    }
    assert_eq!(labels(&suggested(configuration_path, "result \"|\"\n").2), Vec::<&str>::new());

    let other = write(&directory, "other.kdl", "");
    assert_eq!(labels(&suggested(&other, &configuration("code/", "files \"|\"")).2), Vec::<&str>::new());

    let godot = settings("files \"|\"").replace("\"text\"", "\"godot\"");
    assert_eq!(labels(&suggested(settings_path, &godot).2), Vec::<&str>::new());
}

#[test]
fn targets_and_options_are_suggested() {
    let registry = codeshaper::registry();
    let (_directory, paths) = workspace();
    let at = |path: &Path, marked: &str| {
        let found = document(&registry, path, &marked.replacen('|', "", 1), marked.find('|').unwrap());
        labels(&found).into_iter().map(str::to_string).collect::<Vec<_>>()
    };

    assert_eq!(at(&paths[0], "target \"|\"\n"), vec!["godot", "text"]);
    assert_eq!(at(&paths[0], &settings("|").replace("\"text\"", "\"godot\"")), Vec::<String>::new());
    assert!(at(&paths[0], "ast-set \"|\"\n").contains(&"cpp".to_string()));
    assert_eq!(at(&paths[1], "target \"|\"\n"), vec!["code/", "shaping/"]);

    for path in &paths {
        assert_eq!(at(path, &document_text(path, "|")), vec!["files", "exclude"]);
        assert_eq!(at(path, &document_text(path, "fi|")), vec!["files", "exclude"]);
        assert_eq!(at(path, &document_text(path, "files \"a.cpp\"\n    |")), vec!["exclude"]);
    }
}

#[test]
fn configurations_are_known_by_content() {
    let (directory, _) = workspace();

    let engine = write(&directory, "engine.kdl", &configuration("code/", ""));
    assert_eq!(kind(&engine), DocumentKind::Configuration);

    for content in ["", "project \"a\"\n", "project\ntarget\nresult\n", "name \"A\"\ngroup \"root\" {\n    files \"*\"\n}\n"] {
        assert_eq!(kind(&write(&directory, "other.kdl", content)), DocumentKind::Other, "{content}");
    }
    assert_eq!(kind(&directory.path().join("shaping/project.kdl")), DocumentKind::Settings);
}

#[test]
fn patch_files_are_suggested_by_segment() {
    let files: Vec<PathBuf> = ["a.cpp", "src/b.cpp", "src/deep/c.cpp"].iter().map(PathBuf::from).collect();
    let at = |text: &str| {
        let offset = text.find('|').unwrap();
        let text = text.replacen('|', "", 1);
        labels(&target(&text, &(6..text.len() - 2), offset, &files)).into_iter().map(str::to_string).collect::<Vec<_>>()
    };

    assert_eq!(at("file \"|\"\n"), vec!["a.cpp", "src/"]);
    assert_eq!(at("file \"src/|\"\n"), vec!["b.cpp", "deep/"]);
    assert_eq!(at("file \"src/deep/c|\"\n"), vec!["c.cpp"]);
    assert_eq!(at("file \"**/|\"\n"), Vec::<String>::new());
}

#[test]
fn files_must_exist_once() {
    let (_directory, paths) = workspace();

    for path in &paths {
        let text = document_text(path, "files \"a.cpp\" \"gone.cpp\" {\n        \"src\"\n        \"./a.cpp\"\n        /- \"nothing.cpp\"\n        \"src/c.cpp\"\n    }");

        for text in [text.clone(), text.replace('\n', "\r\n")] {
            let diagnostics = check(path, &text);

            assert_eq!(found(&text, &diagnostics), pairs(&[
                ("file-not-found", "\"gone.cpp\""), ("file-is-folder", "\"src\""), ("duplicate-file", "\"./a.cpp\"")
            ]));
            assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
            assert_eq!(diagnostics[2].severity, Some(DiagnosticSeverity::WARNING));
        }

        let text = document_text(path, "files \"a.cpp\" \"b.cpp\"\n    exclude \"b*\"");
        assert_eq!(found(&text, &check(path, &text)), pairs(&[("excluded-file", "\"b.cpp\"")]));

        for body in ["files", "files {\n    }"] {
            let text = document_text(path, body);
            assert_eq!(found(&text, &check(path, &text)), pairs(&[("files-empty", "files")]), "{body}");
        }
    }
}

#[test]
fn unknown_targets_and_options_are_errors() {
    let registry = codeshaper::registry();
    let (_directory, paths) = workspace();

    for path in &paths {
        let text = document_text(path, "include \"*.cpp\"\n    /- other \"a\"");
        let diagnostics = diagnostics(&registry, path, &text);
        assert_eq!(found(&text, &diagnostics), pairs(&[("option-not-found", "include")]));
        assert_eq!(diagnostics[0].message, "target: 'include' is not an option, the options of 'text' are: files, exclude");
    }

    let text = settings("files \"*\"").replace("\"text\"", "\"godot\"");
    assert!(diagnostics(&registry, &paths[0], &text)[0].message.contains("'godot' takes no options"));

    let text = settings("").replace("\"text\"", "\"vcx\"");
    assert_eq!(found(&text, &diagnostics(&registry, &paths[0], &text)), pairs(&[("target-not-found", "\"vcx\"")]));
}

#[test]
fn bracketed_names_are_files() {
    let (directory, paths) = workspace();
    write(&directory, "code/[slug]/[id].tsx", "");

    for path in &paths {
        let text = document_text(path, "files \"[slug]/[id].tsx\"");

        assert_eq!(check(path, &text), vec![]);
        assert_eq!(check_patterns(path, &text, &|| true), Some(vec![]));
        assert_eq!(labels(&suggested(path, &document_text(path, "exclude \"[slug]/|\"")).2), vec!["**/", "*.tsx", "[id].tsx"]);
    }
}

#[test]
fn configuration_folders_must_exist() {
    let (_directory, [_, path]) = workspace();
    let text = configuration("gone/", "files \"nothing.cpp\"");

    assert_eq!(found(&text, &check(&path, &text)), pairs(&[("folder-not-found", "\"gone/\"")]));
    assert_eq!(check(&path, &configuration("code/src", "")), vec![]);
    assert_eq!(check(&path, &configuration("code/a.cpp", "")).len(), 1);
}

#[test]
fn patterns_must_match() {
    let (_directory, paths) = workspace();

    for path in &paths {
        let text = document_text(path, "files \"src/**/*.cpp\" \"*.rs\" \"[\"\n    exclude \"src/deep/**\" \"src/c*\" \"a[\"");
        let diagnostics = check_patterns(path, &text, &|| true).unwrap();

        assert_eq!(found(&text, &diagnostics), pairs(&[
            ("pattern-unmatched", "\"*.rs\""), ("incorrect-pattern", "\"[\""),
            ("exclude-unmatched", "\"src/deep/**\""), ("incorrect-pattern", "\"a[\"")
        ]));
        assert_eq!(diagnostics[2].severity, Some(DiagnosticSeverity::HINT));

        let text = document_text(path, "exclude \"src/**\" \"*.rs\"");
        assert_eq!(found(&text, &check_patterns(path, &text, &|| true).unwrap()), pairs(&[("exclude-unmatched", "\"*.rs\"")]));
    }
}

#[test]
fn patterns_stop_when_told() {
    let (_directory, [path, _]) = workspace();
    let asked = Cell::new(0);
    let keep_going = || { asked.set(asked.get() + 1); asked.get() < 2 };

    assert_eq!(check_patterns(&path, &settings("files \"**/*.cpp\""), &keep_going), None);
    assert_eq!(asked.get(), 2);
}

#[test]
fn defaults_are_checked_for_every_target() {
    let (directory, [path, _]) = workspace();
    write(&directory, "engine/a.cpp", "");
    write(&directory, "engine.kdl", &configuration("engine/", ""));
    let check = |text: &str| {
        let mut diagnostics = check(&path, text);
        diagnostics.extend(check_patterns(&path, text, &|| true).unwrap());
        diagnostics
    };

    let text = settings("files \"a.cpp\" \"b.cpp\" \"src/*\" \"gone.cpp\"\n    exclude \"src/c*\" \"*.rs\"");
    let diagnostics = check(&text);
    assert_eq!(found(&text, &diagnostics), pairs(&[
        ("file-not-found", "\"gone.cpp\""), ("file-not-found", "\"b.cpp\""), ("file-not-found", "\"gone.cpp\""),
        ("exclude-unmatched", "\"*.rs\""), ("pattern-unmatched", "\"src/*\"")
    ]));
    assert!(diagnostics[0].message.ends_with("the target of configuration.kdl"), "{}", diagnostics[0].message);
    assert!(diagnostics[1].message.ends_with("the target of engine.kdl"), "{}", diagnostics[1].message);

    write(&directory, "engine.kdl", &configuration("engine/", "files \"a.cpp\"\n    exclude"));
    assert_eq!(found(&text, &check(&text)), pairs(&[("file-not-found", "\"gone.cpp\""), ("exclude-unmatched", "\"*.rs\"")]));

    let labels = |marked: &str| labels(&suggested(&path, marked).2).into_iter().map(str::to_string).collect::<Vec<_>>();
    write(&directory, "engine/own.rs", "");
    assert!(!labels(&settings("files \"|\"")).contains(&"own.rs".to_string()));
    write(&directory, "engine.kdl", &configuration("engine/", ""));
    assert_eq!(labels(&settings("files \"|\"")), vec!["**/", "*.cpp", "*.h", "*.md", "*.rs", "a.cpp", "b.cpp", "docs/", "own.rs", "src/"]);

    fs::remove_file(directory.path().join("configuration.kdl")).unwrap();
    fs::remove_file(directory.path().join("engine.kdl")).unwrap();
    assert_eq!(check(&text), vec![]);
    assert_eq!(labels(&settings("files \"|\"")), Vec::<String>::new());
}

#[test]
fn configurations_show_what_the_defaults_break() {
    let (directory, [_, path]) = workspace();
    write(&directory, "shaping/project.kdl", &settings("files \"a.cpp\" \"gone.cpp\" \"*.rs\"\n    exclude \"*.py\" \"[\""));
    let check = |text: &str| {
        let mut diagnostics = check(&path, text);
        diagnostics.extend(check_patterns(&path, text, &|| true).unwrap());
        diagnostics
    };

    let text = configuration("code/", "");
    let diagnostics = check(&text);
    assert_eq!(found(&text, &diagnostics), pairs(&[
        ("file-not-found", "\"code/\""), ("pattern-unmatched", "\"code/\""), ("incorrect-pattern", "\"code/\"")
    ]));
    assert!(diagnostics[0].message.starts_with("default files: 'gone.cpp' does not exist"), "{}", diagnostics[0].message);

    let text = configuration("code/", "files \"b.cpp\"\n    exclude \"b*\"");
    assert_eq!(found(&text, &check(&text)), pairs(&[("excluded-file", "\"b.cpp\"")]));
}

#[test]
fn own_folders_are_left_out() {
    let (directory, [_, path]) = workspace();
    write(&directory, "backup/code/a.cpp", "");
    let text = configuration("./", "files \"backup/code/a.cpp\" \"**/a.cpp\"");

    assert_eq!(found(&text, &check(&path, &text)), pairs(&[("own-file", "\"backup/code/a.cpp\"")]));
    let explained = hover(&path, &text, text.find("**/").unwrap(), &|| true).unwrap().0;
    assert!(explained.starts_with("Matches 1 file(s)"), "{explained}");
}

#[test]
fn errors_agree_with_loading() {
    let (_directory, paths) = workspace();
    let mut registry = Registry::default();
    codeshaper_target_text::register(&mut registry);
    let bodies = [
        "files \"a.cpp\"", "files \"gone.cpp\"", "files \"*\"", "files \"a.cpp\" \"*.rs\"", "files \"a.cpp\"\n    exclude \"a*\"",
        "include \"*.cpp\"", "files \"src\"", "files \"a.cpp\" \"a.cpp\"", "files {\n        \"src/c.cpp\"\n        \"x\"\n    }",
        "files \"**/*.cpp\"", "files \"*.rs\"", "files \"[\"", "exclude \"[\"", "files \"*.cpp\"\n    exclude \"src/**\"", "exclude", "files", "files {\n    }", ""
    ];

    for default in bodies {
        for body in bodies {
            let texts = [settings(default), configuration("code/", body)];
            for (path, text) in paths.iter().zip(&texts) { fs::write(path, text).unwrap(); }

            let errors = paths.iter().zip(&texts).any(|(path, text)| {
                let mut diagnostics = diagnostics(&codeshaper::registry(), path, text);
                diagnostics.extend(check_patterns(path, text, &|| true).unwrap());
                diagnostics.iter().any(|diagnostic| diagnostic.severity == Some(DiagnosticSeverity::ERROR))
            });
            let loaded = Operation::from_file(&paths[1]).unwrap().target(&registry);

            assert_eq!(loaded.is_err(), errors, "{default}\n{body}\n{loaded:?}");
        }
    }
}

#[test]
fn entries_open_what_they_name() {
    let (directory, paths) = workspace();
    let opened = |path: &Path, text: &str, needle: &str| {
        opening(path, text, text.find(needle).unwrap() + 1).map(|(range, opening)| (text[range].to_string(), opening))
    };
    let file = |name: &str| Opening::File(directory.path().join(name));

    for path in &paths {
        let text = document_text(path, "files \"a.cpp\" {\n        \"src/c.cpp\"\n        \"gone.cpp\"\n        \"**/*.h\"\n    }\n    exclude \"b.cpp\"");

        assert_eq!(opened(path, &text, "a.cpp"), Some(("a.cpp".to_string(), file("code/a.cpp"))));
        assert_eq!(opened(path, &text, "src/c"), Some(("src/c.cpp".to_string(), file("code/src/c.cpp"))));
        assert_eq!(opened(path, &text, "gone"), None);
        assert_eq!(opened(path, &text, "**/"), Some(("**/*.h".to_string(), Opening::Pattern(Field::Files, "**/*.h".to_string()))));
        assert_eq!(opened(path, &text, "b.cpp"), Some(("b.cpp".to_string(), Opening::Pattern(Field::Exclude, "b.cpp".to_string()))));
        assert_eq!(opened(path, &text, "\"A\""), None);
        assert_eq!(open(path, &text, Field::Files, "**/*.h", &|| true), vec![directory.path().join("code/src/deep/d.h")]);
    }

    let text = configuration("code/", "");
    assert_eq!(opened(&paths[1], &text, "shaping"), Some(("shaping/".to_string(), file("shaping/project.kdl"))));
    assert_eq!(opened(&paths[1], &text, "code"), None);
    assert_eq!(opened(&paths[1], &text, "out/"), None);
    write(&directory, "code/project.godot", "");
    assert_eq!(opened(&paths[1], &text, "code"), Some(("code/".to_string(), file("code/project.godot"))));
}

#[test]
fn patterns_are_explained() {
    let (directory, paths) = workspace();
    let explained = |path: &Path, text: &str, needle: &str| hover(path, text, text.find(needle).unwrap() + 1, &|| true).map(|(value, _)| value);

    for path in &paths {
        let text = document_text(path, "files \"a.cpp\" \"src/**/*.cpp\" \"*.rs\"\n    exclude \"src/c*\" \"b*\"");

        assert!(explained(path, &text, "src/**").unwrap().starts_with("Matches 2 file(s)\n\n- [`src"));
        assert_eq!(explained(path, &text, "*.rs").unwrap(), "Matches no file");
        assert!(explained(path, &text, "src/c*").unwrap().starts_with("Removes 1 of 3 listed file(s)\n\n- [`src"));
        assert_eq!(explained(path, &text, "b*").unwrap(), "Removes no listed file");
        assert_eq!(explained(path, &text, "a.cpp"), None);
        assert_eq!(explained(path, &text, "\"A\""), None);
    }

    let [path, _] = &paths;
    let text = settings("files \"*.cpp\"");
    assert_eq!(explained(path, &text, "files").unwrap(), "Used for every target");
    write(&directory, "engine/a.cpp", "");
    write(&directory, "engine.kdl", &configuration("engine/", ""));
    let both = explained(path, &text, "*.cpp").unwrap();
    assert!(both.contains("**configuration.kdl**: Matches 2 file(s)") && both.contains("**engine.kdl**: Matches 1 file(s)"), "{both}");
    write(&directory, "engine.kdl", &configuration("engine/", "files \"a.cpp\""));
    assert_eq!(explained(path, &text, "files").unwrap(), "Used for every target, except the ones of `engine.kdl` that have their own");

    write(&directory, "shaping/project.kdl", &text);
    let text = configuration("code/", "files \"a.cpp\"\n    exclude \"b*\"");
    assert_eq!(explained(&paths[1], &text, "files").unwrap(), "Used instead of the project's `files`:\n\n- `*.cpp`");
    assert_eq!(explained(&paths[1], &text, "exclude").unwrap(), "The project has no default `exclude`");
}
