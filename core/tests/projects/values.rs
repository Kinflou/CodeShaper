// Standard Uses
use std::collections::BTreeMap;
use std::fs;

// Crate Uses
use crate::operation::{patch, read, registry, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::operation::configuration;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::validation::{called, validate, validate_source, Diagnostic, DiagnosticKind, Severity};
use codeshaper_core::shaping::project::settings::{self, ProjectSettings};
use codeshaper_core::shaping::project::values::{declared, Declared, NotSet, Source};
use miette::{Report, Result};
use tempfile::TempDir;


const SETTINGS: &str = "name \"Run\"\ntarget \"list\"\nast-set \"lines\"\ndescription \"\"\n";

const DECLARED: &str = "constant \"header\" \"Generated\"\nvariable \"prefix\" default=\"my_\"\nvariable \"engine\"\n";

fn parsed(more: &str) -> ProjectSettings {
    settings::from_kdl(&format!("{SETTINGS}{more}")).unwrap()
}

fn errors(more: &str) -> Vec<(String, String)> {
    let text = format!("{SETTINGS}{more}");
    let report = settings::from_kdl(&text).unwrap_err();

    report.related().unwrap()
        .map(|error| {
            let label = error.labels().and_then(|mut labels| labels.next());
            let marked = label.map(|label| text[label.offset()..label.offset() + label.len()].to_string());

            (error.to_string(), marked.unwrap_or_default())
        })
        .collect()
}

fn pairs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
}

fn value(name: &str, value: Option<&str>, source: Source) -> Declared {
    Declared { name: name.to_string(), value: value.map(str::to_string), source }
}

fn directory(more: &str, configured: &str, patches: &[(&str, String)]) -> TempDir {
    let directory = workspace("replace", patches);
    let append = |path: &str, more: &str| {
        let text = read(&directory, path) + more;
        fs::write(directory.path().join(path), text).unwrap();
    };
    append("shaping/project.kdl", more);
    append("configuration.kdl", configured);

    directory
}

fn operation(directory: &TempDir) -> Operation {
    Operation::from_file(&directory.path().join("configuration.kdl")).unwrap()
}

fn run(more: &str, configured: &str, to: &str, set: &[(&str, &str)]) -> Result<String> {
    let patches = [("rename.kdl", patch("", "Line", r"old (\w+)", to))];
    let directory = directory(more, configured, &patches);
    let mut operation = operation(&directory);
    for (name, value) in set { operation.set(name, value)?; }

    let before = read(&directory, "target/a.txt");
    let result = operation.run(&registry());
    let after = read(&directory, "target/a.txt");
    if result.is_err() { assert_eq!(before, after); }

    result.map(|_| after)
}

fn chain(error: Report) -> String {
    error.chain().map(|error| error.to_string()).collect::<Vec<_>>().join(": ")
}


#[test]
fn declarations_are_read() {
    let settings = parsed(DECLARED);

    assert_eq!(settings.constants.len(), 1);
    assert_eq!((settings.constants[0].name.as_str(), settings.constants[0].value.as_str()), ("header", "Generated"));
    let variables: Vec<(&str, Option<&str>)> = settings.variables.iter()
        .map(|variable| (variable.name.as_str(), variable.default.as_deref()))
        .collect();
    assert_eq!(variables, vec![("prefix", Some("my_")), ("engine", None)]);

    assert!(parsed("").constants.is_empty() && parsed("").variables.is_empty());
    assert_eq!(parsed("constant \"a-B_9!\" \"\"\n").constants[0].value, "");
    assert_eq!(parsed("variable \"a\" default=\"\"\n").variables[0].default.as_deref(), Some(""));
}

#[test]
fn wrong_declarations_are_reported() {
    let pair = |message: &str, marked: &str| vec![(message.to_string(), marked.to_string())];

    for name in ["", "a b", "é", "a.b", "a]"] {
        let message = format!("'{name}' can't be called, names have letters, digits, '_', '-' and '!'");
        assert_eq!(errors(&format!("variable \"{name}\"\n")), pair(&message, &format!("\"{name}\"")));
    }

    assert_eq!(errors("constant \"a\"\n"), pair("constant 'a' has no value", "constant"));
    assert_eq!(errors("constant\n"), pair("the constant has no name", "constant"));
    assert_eq!(errors("variable\n"), pair("the variable has no name", "variable"));
    assert_eq!(errors("constant \"a\" 1\n"), pair("expected text in quotes", "1"));
    assert_eq!(errors("variable \"a\" default=true\n"), pair("expected text in quotes", "true"));
    assert_eq!(errors("constant \"a\" \"b\" \"c\"\n"), pair("a constant takes its name and its value", "\"c\""));
    assert_eq!(errors("constant \"a\" \"b\" c=\"d\"\n"), pair("a constant takes its name and its value", "c"));
    assert_eq!(errors("variable \"a\" \"b\"\n"), pair("a variable takes its name, its value goes in default=\"…\"", "\"b\""));
    assert_eq!(errors("variable \"a\" value=\"b\"\n"), pair("a variable takes its name and default=\"…\"", "value"));
    assert_eq!(errors("variable \"a\" {\n}\n"), pair("a variable takes its name, its value goes in default=\"…\"", "variable"));
}

#[test]
fn names_are_declared_once() {
    for second in ["variable \"a\"", "constant \"a\" \"c\""] {
        let found = errors(&format!("constant \"a\" \"b\"\nvariable \"b\"\n{second}\n"));
        assert_eq!(found, vec![("the project already declares 'a'".to_string(), "\"a\"".to_string())]);
    }
}

#[test]
fn values_come_from_the_first_that_has_one() {
    let settings = parsed(DECLARED);
    let found = |configured: &[(&str, &str)], set: &[(&str, &str)]| declared(&settings, &pairs(configured), &pairs(set));
    let header = value("header", Some("Generated"), Source::Constant);

    assert_eq!(found(&[], &[]), vec![header.clone(), value("prefix", Some("my_"), Source::Default), value("engine", None, Source::Missing)]);
    assert_eq!(found(&[("prefix", "x_"), ("engine", "4")], &[]), vec![
        header.clone(), value("prefix", Some("x_"), Source::Configuration), value("engine", Some("4"), Source::Configuration)
    ]);
    assert_eq!(found(&[("prefix", "x_"), ("engine", "4")], &[("prefix", "")]), vec![
        header, value("prefix", Some(""), Source::Set), value("engine", Some("4"), Source::Configuration)
    ]);
}

#[test]
fn configurations_set_variables() {
    let text = |variables: &str| format!("name \"o\"\nproject \"p\"\ntarget \"t\"\noutput \"o\"\nresult \"replace\"\n{variables}");
    let found = |variables: &str| configuration::from_kdl(&text(variables)).map(|configuration| configuration.variables);

    assert_eq!(found("").unwrap(), pairs(&[]));
    assert_eq!(found("variables engine=\"4\" a-b=\"\"\n").unwrap(), pairs(&[("engine", "4"), ("a-b", "")]));
    assert!(found("variables engine=4\n").is_err());
    assert!(found("variables \"engine\"\n").is_err());
}

#[test]
fn runs_insert_values() {
    assert_eq!(run(DECLARED, "", "#[prefix]$[0] #[header]", &[]).unwrap(), "my_one Generated\nkeep\n");
    assert_eq!(run(DECLARED, "variables prefix=\"x_\"\n", "#[prefix]$[0]", &[]).unwrap(), "x_one\nkeep\n");
    assert_eq!(run(DECLARED, "variables prefix=\"x_\"\n", "#[prefix]$[0]", &[("prefix", "é𝄞 ")]).unwrap(), "é𝄞 one\nkeep\n");
    assert_eq!(run(DECLARED, "", "[#[prefix]]", &[("prefix", "")]).unwrap(), "[]\nkeep\n");
}

#[test]
fn values_are_plain_text() {
    let value = r##"#[header] $[0] @[0] "quoted" \ $1 ${0}"##;
    assert_eq!(run(DECLARED, "", "#[engine]", &[("engine", value)]).unwrap(), format!("{value}\nkeep\n"));
}

#[test]
fn line_endings_follow_the_file() {
    let patches = [("rename.kdl", patch("", "Line", r"old (\w+)", "#[engine]#[header]"))];
    for (source, expected) in [("old one\nkeep\n", "a\nb\nc\nkeep\n"), ("old one\r\nkeep\r\n", "a\r\nb\r\nc\r\nkeep\r\n")] {
        for value in ["a\nb\n", "a\r\nb\r\n"] {
            for constant in ["constant \"header\" r\"c\"\nvariable \"engine\"\n", "constant \"header\" \"c\"\r\nvariable \"engine\"\r\n"] {
                let directory = directory(constant, "", &patches);
                fs::write(directory.path().join("target/a.txt"), source).unwrap();
                let mut operation = operation(&directory);
                operation.set("engine", value).unwrap();
                operation.run(&registry()).unwrap();

                assert_eq!(read(&directory, "target/a.txt"), expected);
            }
        }
    }
}

#[test]
fn every_expression_takes_values() {
    let actions = r##"
        builder "b" {
            location "Line"
            match "(old)"
            build "#[prefix]$[0]#[nested]"
            actions {
                maker "nested" {
                    make "(#[header])"
                }
            }
        }
        maker "m" {
            make "@[0]-#[prefix]"
        }
        switch "s" {
            cases my_="found"
            index "#[prefix]"
        }
        list "l" {
            values "a" "b"
            index "#[number]"
        }
        replacer {
            location "Line"
            from "old one"
            to "#[b] #[m(#[header])] #[s] #[l] #[m(\"#[prefix]\")]"
        }
    "##;
    let content = format!("file \"*.txt\"\nactions {{\n{actions}\n}}\n");
    let directory = directory(&format!("{DECLARED}constant \"number\" \"1\"\n"), "", &[("a.kdl", content)]);
    operation(&directory).run(&registry()).unwrap();

    assert_eq!(read(&directory, "target/a.txt"), "my_old(Generated) Generated-my_ found b my_-my_\nkeep\n");
}

#[test]
fn previews_traces_and_sessions_agree() {
    let content = patch("", "Line", r"old (\w+)", "#[prefix]$[0]");
    let directory = directory(DECLARED, "", &[("rename.kdl", content.clone())]);
    let mut operation = operation(&directory);
    operation.set("prefix", "x_").unwrap();
    let patch = Patch::from_kdl("", &content).unwrap();

    let changes = operation.preview(&registry(), "rename.kdl", &patch).unwrap();
    assert_eq!(changes[0].result.as_ref().unwrap(), "x_one\nkeep\n");

    let trace = operation.trace(&registry(), "rename.kdl", &patch, 9).unwrap();
    assert_eq!(trace.action("replacer #1").unwrap().samples[0].output, "x_one");

    let registry = registry();
    let preview = operation.session(&registry).unwrap().preview().unwrap();
    assert_eq!(preview.files.iter().map(|file| file.result.as_str()).collect::<Vec<_>>(), vec!["x_one\nkeep\n"]);
    assert_eq!(read(&directory, "target/a.txt"), "old one\nkeep\n");
}

#[test]
fn called_variables_need_a_value() {
    let error = run(DECLARED, "", "#[engine]", &[]).unwrap_err();
    assert_eq!(error.to_string(), "Variable 'engine' has no value, set it in the configuration or with --set, or give it a default in project.kdl");
    assert_eq!(error.downcast_ref::<NotSet>().unwrap().names, vec!["engine"]);

    let error = run("variable \"b\"\nvariable \"a\"\nvariable \"c\"\n", "", "#[a]#[b]", &[]).unwrap_err();
    assert_eq!(error.to_string(), "Variables 'b', 'a' have no value, set them in the configuration or with --set, or give them a default in project.kdl");

    assert!(run(DECLARED, "", "#[prefix]", &[]).is_ok());
    assert!(run(DECLARED, "variables engine=\"\"\n", "#[engine]", &[]).is_ok());

    let content = patch("", "Line", "old", "#[engine]");
    let found = directory(DECLARED, "", &[]);
    assert!(operation(&found).preview(&registry(), "", &Patch::from_kdl("", &content).unwrap()).unwrap_err().downcast_ref::<NotSet>().is_some());
}

#[test]
fn only_patches_that_run_need_values() {
    let calling = patch("", "Line", "old", "#[engine]");
    for content in [calling.replace("enabled true", "enabled false"), calling.replace("**/*.txt", "*.none")] {
        let directory = directory(DECLARED, "", &[("a.kdl", content)]);
        assert!(operation(&directory).run(&registry()).is_ok());
    }
}

#[test]
fn only_variables_are_set() {
    let message = |more: &str, configured: &str, set: &[(&str, &str)]| chain(run(more, configured, "x", set).unwrap_err());

    assert_eq!(message(DECLARED, "", &[("missing", "a")]), "project.kdl declares no variable named 'missing'");
    assert_eq!(message(DECLARED, "", &[("header", "a")]), "'header' is a constant, constants can't be set");
    assert_eq!(
        message(DECLARED, "variables missing=\"a\"\n", &[]),
        "Operation 'Run' sets 'missing' in its variables: project.kdl declares no variable named 'missing'"
    );
    assert_eq!(
        message(DECLARED, "variables header=\"a\"\n", &[]),
        "Operation 'Run' sets 'header' in its variables: 'header' is a constant, constants can't be set"
    );

    let directory = directory(DECLARED, "variables header=\"a\"\n", &[]);
    let operation = operation(&directory);
    assert!(operation.check(&registry()).is_err());
    assert!(operation.preview(&registry(), "", &Patch::from_kdl("", &patch("", "Line", "a", "b")).unwrap()).is_err());
}

fn found(actions: &str, declared: &[Declared]) -> Vec<(DiagnosticKind, Severity, String, String)> {
    let content = format!("file \"*\"\nactions {{\n{actions}\n}}\n");
    let (patch, source) = Patch::parse("", &content).unwrap();
    let found = validate_source(&patch, &source, [], declared);
    let plain: Vec<Diagnostic> = found.iter().map(|diagnostic| Diagnostic { span: None, ..diagnostic.clone() }).collect();
    assert_eq!(validate(&patch, [], declared), plain);

    found.into_iter()
        .map(|diagnostic| {
            let span = diagnostic.span.unwrap();
            (diagnostic.kind, diagnostic.severity, diagnostic.message, content[span.offset()..span.offset() + span.len()].to_string())
        })
        .collect()
}

#[test]
fn patches_are_checked_against_the_names() {
    let names = declared(&parsed(DECLARED), &pairs(&[]), &pairs(&[]));
    let replacer = |to: &str| format!("replacer {{\nlocation \"Line\"\nfrom \"a\"\nto \"{to}\"\n}}\n");
    let one = |kind: DiagnosticKind, severity: Severity, message: &str, marked: &str| vec![(kind, severity, message.to_string(), marked.to_string())];

    assert_eq!(found(&replacer("#[prefix]#[header]"), &names), vec![]);
    assert_eq!(found(&replacer("#[prefix]()#[prefix](a)#[prefix]($[0])#[header()]").replace("\"a\"", "\"(a)\""), &names), vec![]);
    assert_eq!(found(&replacer("#[prefix]"), &[]), one(DiagnosticKind::ActionNotFound, Severity::Error, "No action named 'prefix'", "#[prefix]"));
    assert_eq!(found(&replacer("#[other]"), &names), one(DiagnosticKind::ActionNotFound, Severity::Error, "No action named 'other'", "#[other]"));
    assert_eq!(found(&replacer("a #[engine]"), &names), one(DiagnosticKind::VariableNotSet, Severity::Warning, "Variable 'engine' has no value", "#[engine]"));

    let message = |kind: &str, name: &str| format!("'#[{name}]' is a {kind}, it takes no arguments, but is called with 1");
    for (kind, name) in [("variable", "prefix"), ("constant", "header")] {
        assert_eq!(
            found(&replacer(&format!("#[{name}('a')]")), &names),
            one(DiagnosticKind::IncorrectArgs, Severity::Error, &message(kind, name), &format!("#[{name}"))
        );
    }
}

#[test]
fn actions_can_not_take_the_names() {
    let names = declared(&parsed(DECLARED), &pairs(&[]), &pairs(&[]));
    let calls = "replacer {\nlocation \"Line\"\nfrom \"a\"\nto \"#[prefix]#[header]#[outer]\"\n}\n";
    let maker = |name: &str| format!("maker \"{name}\" {{\nmake \"a\"\n}}\n");
    let nested = format!("builder \"outer\" {{\nlocation \"Line\"\nmatch \"a\"\nbuild \"#[header]\"\nactions {{\n{}}}\n}}\n", maker("header"));

    for (actions, name, kind) in [
        (maker("prefix") + &maker("outer"), "prefix", "variable"),
        (nested, "header", "constant"),
        (maker("outer") + "replacer \"prefix\" {\nlocation \"Line\"\nfrom \"a\"\nto \"b\"\n}\n", "prefix", "variable")
    ] {
        assert_eq!(found(&(actions + calls), &names), vec![(
            DiagnosticKind::DuplicateAction, Severity::Error,
            format!("The name '{name}' is already a {kind} of the project"), format!("\"{name}\"")
        )]);
    }
}

#[test]
fn called_names_are_found() {
    let content = "file \"*\"\nactions {\nmaker \"a\" {\nmake \"#[b(#[c])]\"\nprepare \"(\"\n}\nreplacer {\nlocation \"x\"\nfrom \"y\"\nto \"#[a]#[\"\n}\n}\n";
    let called = called(&Patch::from_kdl("", content).unwrap());

    assert_eq!(called.into_iter().collect::<Vec<_>>(), vec!["a", "b", "c"]);
}

#[test]
fn checks_warn_about_missing_values() {
    let directory = directory(DECLARED, "", &[("a.kdl", patch("", "Line", "old", "#[engine]#[prefix]"))]);
    let mut operation = operation(&directory);
    let found = |operation: &Operation| operation.check(&registry()).unwrap().into_iter()
        .map(|(_, diagnostic)| (diagnostic.kind, diagnostic.severity))
        .collect::<Vec<_>>();

    assert_eq!(found(&operation), vec![(DiagnosticKind::VariableNotSet, Severity::Warning)]);
    operation.set("engine", "4").unwrap();
    assert_eq!(found(&operation), vec![]);
}

#[test]
fn projects_give_their_values_without_an_operation() {
    let directory = directory(DECLARED, "variables engine=\"4\"\n", &[]);
    let project = directory.path().join("shaping");
    let found = |set: &[(&str, &str)]| Operation::declared_for(&project, &pairs(set));

    assert_eq!(found(&[("prefix", "x")]).unwrap(), vec![
        value("header", Some("Generated"), Source::Constant), value("prefix", Some("x"), Source::Set), value("engine", Some("4"), Source::Configuration)
    ]);
    assert!(found(&[("header", "x")]).is_err());

    fs::remove_file(directory.path().join("configuration.kdl")).unwrap();
    assert_eq!(found(&[]).unwrap()[2], value("engine", None, Source::Missing));
}
