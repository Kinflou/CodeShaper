// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::pattern::groups;
use codeshaper_core::shaping::patch::validation::{validate, validate_source, Diagnostic, DiagnosticKind, Severity};
use miette::{bail, Result};


struct LineBackend;

impl LanguageBackend for LineBackend {
    fn name(&self) -> &str { "lines" }

    fn locate(&self, _source: &str, _location: &str) -> Result<Vec<Range<usize>>> {
        Ok(vec![])
    }

    fn check_location(&self, location: &str) -> Result<()> {
        if location != "Line" {
            bail!("'{location}' is not a lines location")
        }

        Ok(())
    }
}

fn content(actions: &str) -> String {
    format!("enabled true\nalias \"lines\"\nfile \"*.txt\"\n\nactions {{\n{actions}\n}}\n")
}

fn patch(actions: &str) -> Patch {
    Patch::from_kdl("test.kdl", &content(actions)).unwrap()
}

fn check_source(actions: &str) -> (String, Vec<Diagnostic>) {
    let content = content(actions);
    let (patch, source) = Patch::parse("test.kdl", &content).unwrap();
    let diagnostics = validate_source(&patch, &source, Some(&LineBackend));

    (content, diagnostics)
}

fn spanned<'a>(content: &'a str, diagnostic: &Diagnostic) -> (usize, &'a str) {
    let span = diagnostic.span.unwrap();

    (span.offset(), &content[span.offset()..span.offset() + span.len()])
}

fn check(actions: &str) -> Vec<Diagnostic> {
    validate(&patch(actions), Some(&LineBackend))
}

fn kinds(diagnostics: &[Diagnostic]) -> Vec<(DiagnosticKind, &str, &str)> {
    diagnostics.iter()
        .map(|diagnostic| (diagnostic.kind, diagnostic.action.as_str(), diagnostic.field))
        .collect()
}

fn replacer(location: &str, from: &str, to: &str) -> String {
    format!(r#"replacers {{
        replacer {{
            location "{location}"
            from r"{from}"
            to "{to}"
        }}
    }}"#)
}

const NAMES_BUILDER: &str = r#"builders {
        builder "names" {
            location "Line"
            match r"(\w+)"
            build "$[0]"
        }
    }"#;


#[test]
fn clean_patch() {
    let actions = format!("{NAMES_BUILDER}\n{}", replacer("Line", r"old (\w+)", "new $[0] #[names]"));

    assert_eq!(check(&actions), vec![]);
}

#[test]
fn incorrect_regex() {
    let diagnostics = check(&replacer("Line", "old (", "new $[0]"));

    assert_eq!(kinds(&diagnostics), vec![(DiagnosticKind::IncorrectRegex, "replacer #1", "from")]);
    assert_eq!(diagnostics[0].severity, Severity::Error);
}

#[test]
fn unknown_flags() {
    let actions = replacer("Line", "old (\\w+)", "new $[0]").replace("to \"", "flags \"iq\"\n            to \"");
    let (content, diagnostics) = check_source(&actions);

    assert_eq!(kinds(&diagnostics), vec![(DiagnosticKind::IncorrectRegex, "replacer #1", "flags")]);
    assert_eq!(spanned(&content, &diagnostics[0]).1, r#""iq""#);
}

#[test]
fn unclosed_call() {
    let diagnostics = check(&replacer("Line", "old (\\w+)", "new $[0] #[names"));

    assert_eq!(kinds(&diagnostics), vec![(DiagnosticKind::IncorrectExpression, "replacer #1", "to")]);
    assert!(diagnostics[0].message.contains("'#[names' is not a complete call"));
}

#[test]
fn location_not_found() {
    let actions = replacer("Missing", "old", "new");

    assert_eq!(kinds(&check(&actions)), vec![(DiagnosticKind::LocationNotFound, "replacer #1", "location")]);
    assert_eq!(validate(&patch(&actions), None), vec![]);
}

#[test]
fn action_not_found() {
    let diagnostics = check(&replacer("Line", "old", "#[nope]"));

    assert_eq!(kinds(&diagnostics), vec![(DiagnosticKind::ActionNotFound, "replacer #1", "to")]);
    assert!(diagnostics[0].message.contains("'nope'"));
}

#[test]
fn locals_out_of_range_or_named() {
    let diagnostics = check(&replacer("Line", r"old (\w+)", "$[1] $[name]"));

    assert_eq!(kinds(&diagnostics), vec![
        (DiagnosticKind::IncorrectArgs, "replacer #1", "to"),
        (DiagnosticKind::IncorrectArgs, "replacer #1", "to"),
    ]);
    assert!(diagnostics[0].message.contains("needs 2 capture group(s), but the regex has 1"));
}

#[test]
fn arguments_outside_makers_and_indexes() {
    assert_eq!(
        kinds(&check(&replacer("Line", "old", "@[0]"))),
        vec![(DiagnosticKind::IncorrectArgs, "replacer #1", "to")]
    );
}

#[test]
fn makers_take_arguments_but_need_prepare_for_locals() {
    let actions = format!(r#"makers {{
        maker "wrap" {{
            make "<@[0]>"
        }}
        maker "local" {{
            make "$[0]"
        }}
    }}
    {}"#, replacer("Line", r"old (\w+)", "#[wrap]($[0]) #[local]"));

    assert_eq!(kinds(&check(&actions)), vec![(DiagnosticKind::IncorrectArgs, "local", "make")]);
}

#[test]
fn duplicate_and_unused_actions() {
    let actions = format!(r#"{NAMES_BUILDER}
    {}"#, replacer("Line", "old", "new").replace("from", r#"actions {
                builders {
                    builder "names" {
                        location "Line"
                        match r"(\w+)"
                        build "$[0]"
                    }
                }
            }
            from"#));

    let diagnostics = check(&actions);

    assert_eq!(kinds(&diagnostics), vec![
        (DiagnosticKind::DuplicateAction, "replacer #1/names", "name"),
        (DiagnosticKind::UnusedAction, "names", "name"),
    ]);
    assert_eq!(diagnostics[1].severity, Severity::Warning);
}

#[test]
fn spans_point_at_values() {
    let (content, diagnostics) = check_source(&replacer("Missing", "old (", "new #[gone]"));

    assert_eq!(kinds(&diagnostics), vec![
        (DiagnosticKind::LocationNotFound, "replacer #1", "location"),
        (DiagnosticKind::IncorrectRegex, "replacer #1", "from"),
        (DiagnosticKind::ActionNotFound, "replacer #1", "to"),
    ]);
    assert_eq!(spanned(&content, &diagnostics[0]).1, r#""Missing""#);
    assert_eq!(spanned(&content, &diagnostics[1]).1, "(");
    assert_eq!(spanned(&content, &diagnostics[2]).1, "#[gone]");
}

#[test]
fn duplicate_points_at_the_second_definition() {
    let actions = format!(r#"{NAMES_BUILDER}
    {}"#, replacer("Line", "old", "new").replace("from", r#"actions {
                builders {
                    builder "names" {
                        location "Line"
                        match r"(\w+)"
                        build "$[0]"
                    }
                }
            }
            from"#));

    let (content, diagnostics) = check_source(&actions);
    let first = content.find(r#""names""#).unwrap();
    let second = content.rfind(r#""names""#).unwrap();

    assert_eq!(spanned(&content, &diagnostics[0]), (second, r#""names""#));
    assert_eq!(spanned(&content, &diagnostics[1]), (first, r#""names""#));
}

#[test]
fn calls_pass_enough_arguments() {
    let actions = format!(r#"{NAMES_BUILDER}
    makers {{
        maker "pair" {{
            make "@[0]=@[1]"
        }}
    }}
    {}"#, replacer("Line", r"old (\w+)", "#[pair]($[0], $[0]) #[pair]($[0]) #[names]($[0])"));

    let diagnostics = check(&actions);

    assert_eq!(kinds(&diagnostics), vec![
        (DiagnosticKind::IncorrectArgs, "replacer #1", "to"),
        (DiagnosticKind::IncorrectArgs, "replacer #1", "to"),
    ]);
    assert!(diagnostics[0].message.contains("'#[pair]' is called with 1 argument(s), but uses '@[1]'"));
    assert!(diagnostics[1].message.contains("'#[names]' takes no arguments, but is called with 1"));
}

fn underlined(from: &str, to: &str) -> Vec<String> {
    let actions = replacer("Line", "FROM", "TO").replace(r#"r"FROM""#, from).replace(r#""TO""#, to);
    let (content, diagnostics) = check_source(&actions);

    diagnostics.iter().map(|diagnostic| spanned(&content, diagnostic).1.to_string()).collect()
}

#[test]
fn spans_allow_for_escapes() {
    assert_eq!(underlined(r#"r"(\w+)""#, r##""\t$[0();\n""##), vec!["$[0"]);
    assert_eq!(underlined(r#"r"(\w+)""#, r##""\u{e9}\"é\\ $[5] $[0]""##), vec!["$[5]"]);
    assert_eq!(underlined(r#""(\\w+) (""#, r#""$[0]""#), vec!["("]);
}

#[test]
fn spans_allow_for_raw_and_multiline_strings() {
    assert_eq!(underlined(r#"r"(\w+)""#, "r##\"a \"#\n  #[gone] é $[7]\"##"), vec!["$[7]", "#[gone]"]);
    assert_eq!(underlined("r#\"(\\w+)\n[z\"#", r#""x""#), vec!["["]);
}

#[test]
fn each_wrong_call_has_its_own_span() {
    assert_eq!(underlined(r#"r"old""#, r##""#[one]($[0]) #[two]""##), vec!["$[0]", "#[one]", "#[two]"]);
    assert_eq!(underlined(r#"r"old""#, r##""a @[0] b""##), vec!["@[0]"]);
}

#[test]
fn groups_have_ranges_and_names() {
    let pattern = r"(a)(?:b)(?P<n>c(d))[(]\(e\)(?i:f)";
    let found: Vec<(&str, Option<String>)> = groups(pattern, "").unwrap().into_iter()
        .map(|group| (&pattern[group.range], group.name))
        .collect();

    assert_eq!(found, vec![("(a)", None), ("(?P<n>c(d))", Some("n".to_string())), ("(d)", None)]);
    assert_eq!(groups("(a # (b)\n)", "x").unwrap().len(), 1);
    assert_eq!(groups("(a", ""), None);
}
