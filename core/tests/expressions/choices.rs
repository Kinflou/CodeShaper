// Standard Uses

// Crate Uses
use crate::expressions::arguments::shaped;

// External Uses
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::validation::{called, validate_source};


const HELPERS: &str = r#"
    maker "wrap" {
        make "<@[0]>"
    }
    builder "line" {
        location "Line"
        match r"(\w+)"
        build "$[0]"
    }
"#;

fn switch(cases: &str, default: &str) -> String {
    format!("{HELPERS}\nswitch \"s\" {{\ncases {cases}\nindex \"@[0]\"\n{default}\n}}\n")
}

fn list(values: &str, default: &str) -> String {
    format!("{HELPERS}\nlist \"l\" {{\nvalues {values}\nindex \"@[0]\"\n{default}\n}}\n")
}

fn found(action: &str) -> Vec<(String, String)> {
    let content = format!("file \"*\"\nactions {{\n{action}\nreplacer {{\nlocation \"Line\"\nfrom \"a\"\nto \"#[s('x')]#[l(0)]\"\n}}\n}}\n");
    let (patch, source) = Patch::parse("", &content).unwrap();

    validate_source(&patch, &source, [], &[]).into_iter()
        .filter(|diagnostic| diagnostic.message != "No action named 's'" && diagnostic.message != "No action named 'l'" && diagnostic.field != "name")
        .map(|diagnostic| {
            let span = diagnostic.span.unwrap();
            (format!("{}: {}", diagnostic.field, diagnostic.message), content[span.offset()..span.offset() + span.len()].to_string())
        })
        .collect()
}


#[test]
fn picked_texts_are_run() {
    let calls = switch(r##"one="#[wrap(@[0])]" two="#[line]-@[1]" "a b"="plain""##, "default \"@[0] #[wrap('@[1]')]\"");
    assert_eq!(shaped(&calls, "#[s($[0], 'x')]"), "<one>");
    assert_eq!(shaped(&calls, "#[s($[1], 'x')]"), "one-x");
    assert_eq!(shaped(&calls, "#[s('a b', 'x')]"), "plain");
    assert_eq!(shaped(&calls, "#[s('é', 'x')]"), "é <x>");

    let calls = list(r##""#[wrap(@[0])]" "#[line]" "c""##, "default \"none of @[0]\"");
    assert_eq!(shaped(&calls, "#[l(0)] #[l(1)] #[l(2)] #[l(3)]"), "<0> one c none of 3");
}

#[test]
fn only_the_picked_text_matters() {
    let calls = switch(r##"one="1" two="#[t('gone')]""##, "") + "switch \"t\" {\ncases a=\"b\"\nindex \"@[0]\"\n}\n";
    assert_eq!(shaped(&calls, "#[s($[0])]"), "1");
    assert_eq!(shaped(&calls, "#[s($[1])]"), "one two");
}

#[test]
fn choices_call_each_other() {
    let calls = switch(r##"one="#[l(1)]" two="2""##, "") + "list \"l\" {\nvalues \"a\" \"#[s('two')]!\"\nindex \"@[0]\"\n}\n";
    let error = found(&calls);

    assert!(error.iter().all(|(message, _)| message.contains("calls back to this action")), "{error:?}");
    assert_eq!(error.iter().map(|(_, marked)| marked.as_str()).collect::<Vec<_>>(), vec!["#[s", "#[l"]);
}

#[test]
fn texts_without_calls_stay_as_they_are() {
    for text in ["", "a $0 ${x} #{y} @z", "é\\t\\\"𝄞", "[0] #", "it's"] {
        let plain = text.replace("\\t", "\t").replace("\\\"", "\"");

        assert_eq!(shaped(&switch(&format!("one=\"{text}\""), ""), "#[s($[0])]"), plain);
        assert_eq!(shaped(&list(&format!("\"{text}\""), ""), "#[l(0)]"), plain);
        assert_eq!(shaped(&switch("x=\"\"", &format!("default \"{text}\"")), "#[s($[0])]"), plain);
    }
}

#[test]
fn problems_point_at_their_text() {
    let problems = switch(r##"a="ok" b="x #[gone]" c="$[0]" d="#[wrap" e=r"é @[x]""##, "default \"#[lost]\"");
    assert_eq!(found(&problems), vec![
        ("cases: '$[0]' has no capture group to refer to: switches and lists have no regex".to_string(), "$[0]".to_string()),
        ("cases: '#[wrap' is not a complete call: expected a name followed by ']'".to_string(), "#[wrap".to_string()),
        ("cases: '@[x]' must be an argument number".to_string(), "@[x]".to_string()),
        ("cases: No action named 'gone'".to_string(), "#[gone]".to_string()),
        ("default: No action named 'lost'".to_string(), "#[lost]".to_string())
    ]);

    let problems = list(r##""ok" "\t#[gone]" "@[3]""##, "default \"$[n]\"");
    assert_eq!(found(&problems), vec![
        ("default: '$[n]' has no capture group to refer to: switches and lists have no regex".to_string(), "$[n]".to_string()),
        ("to: '#[l]' is called with 1 argument(s), but uses '@[3]'".to_string(), "#[l".to_string()),
        ("values: No action named 'gone'".to_string(), "#[gone]".to_string())
    ]);
}

#[test]
fn calls_in_choices_are_calls() {
    let content = format!("file \"*\"\nactions {{\n{}\n{}\n}}\n", switch(r##"a="#[one]" b="#[two('#[three]')]""##, "default \"#[four]\""), "list \"l\" {\nvalues \"#[five]\"\ndefault \"#[six]\"\n}\n");
    let patch = Patch::from_kdl("", &content).unwrap();

    assert_eq!(called(&patch).into_iter().collect::<Vec<_>>(), vec!["five", "four", "one", "six", "three", "two"]);
}
