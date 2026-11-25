// Standard Uses

// Crate Uses
use crate::operation::{registry, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::trace::Trace;
use codeshaper_core::shaping::patch::validation::validate_source;
use miette::Report;


const MIXED: &str = r##"
replacer {
    location "Line"
    from r"old (\w+)"
    to "#[second]($[0])"
}
maker "second" {
    make "#[kind]($[0]) 2"
    prepare r"(\w+)"
}
builder "names" {
    location "Line"
    match r"^(\w+)$"
    build "<#[inner]>"
    collect true
    actions {
        switch "deep" {
            index "@[0]"
            cases a="x"
            default "y"
        }
        builder "inner" {
            location "Line"
            match r"(\w+)"
            build "#[deep]($[0])"
        }
    }
}
switch "kind" {
    index "@[0]"
    cases one="1"
}
replacer {
    location "Line"
    from r"^1 (\d)$"
    to "#[order]($[0]) #[names]"
    actions {
        list "order" {
            index "@[0]"
            values "a" "b" "c"
        }
    }
}
list "unused" {
    index "@[0]"
    values "z"
}
"##;

const GROUPED: [&str; 5] = ["builder", "replacer", "maker", "switch", "list"];

fn content(actions: &str) -> String {
    format!("enabled true\nalias \"lines\"\nfile \"*.txt\"\nactions {{{actions}}}\n")
}

fn blocks(actions: &str) -> Vec<String> {
    let mut blocks: Vec<String> = vec![];

    for line in actions.trim().lines() {
        if line.starts_with(|character: char| character.is_ascii_lowercase()) { blocks.push(String::new()) }
        *blocks.last_mut().unwrap() += &format!("{line}\n");
    }

    blocks
}

fn grouped(actions: &str) -> String {
    let blocks = blocks(actions);
    let sorted = GROUPED.iter().flat_map(|name| blocks.iter().filter(move |block| block.starts_with(name)));

    format!("\n{}", sorted.cloned().collect::<String>())
}

fn traced(patch: &Patch) -> Trace {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

    Trace { directory: Default::default(), ..operation.trace(&registry(), "", patch, 9).unwrap() }
}

fn preview(patch: &Patch) -> String {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

    operation.preview(&registry(), "", patch).unwrap().remove(0).result.unwrap()
}

fn errors(text: &str) -> Vec<(String, String)> {
    let report: Report = Patch::from_kdl("mixed.kdl", text).unwrap_err();

    report.related().unwrap()
        .map(|error| {
            let label = error.labels().and_then(|mut labels| labels.next()).unwrap();

            (error.to_string(), text[label.offset()..label.offset() + label.len()].to_string())
        })
        .collect()
}


#[test]
fn the_order_between_types_means_nothing() {
    let (mixed, sorted) = (content(MIXED), content(&grouped(MIXED)));
    assert_ne!(mixed, sorted);

    let (mixed, sorted) = (Patch::from_kdl("", &mixed).unwrap(), Patch::from_kdl("", &sorted).unwrap());

    assert_eq!(mixed, sorted);
    assert_eq!(preview(&mixed), "c <y><y>\nkeep\n");
    assert_eq!((preview(&mixed), traced(&mixed)), (preview(&sorted), traced(&sorted)));
}

#[test]
fn replacers_run_and_count_as_written() {
    let patch = Patch::from_kdl("", &content(MIXED)).unwrap();
    let trace = traced(&patch);

    assert_eq!((trace.action("replacer #1").unwrap().matches, trace.action("replacer #2").unwrap().matches), (1, 1));
    assert_eq!(trace.action("replacer #2").unwrap().samples[0].input, "1 2");
    assert_eq!(trace.action("replacer #2/order").unwrap().calls, 1);
    assert_eq!(trace.action("names/inner").unwrap().calls, 2);
    assert_eq!(trace.action("names/deep").unwrap().keys.iter().collect::<Vec<_>>(), vec!["1"]);

    let swapped = blocks(MIXED).into_iter().rev().collect::<String>();
    assert_eq!(preview(&Patch::from_kdl("", &content(&format!("\n{swapped}"))).unwrap()), "1 2\nkeep\n");
}

#[test]
fn actions_may_be_empty() {
    for actions in ["", "\n", "\n/-maker \"gone\" {\n    make 1\n}\n"] {
        let patch = Patch::from_kdl("", &content(actions)).unwrap();

        assert_eq!(patch.actions, Default::default());
        assert_eq!(preview(&patch), "old one\nkeep\n");
    }
}

#[test]
fn containers_are_unexpected() {
    for name in ["builders", "replacers", "makers", "switchers", "lists"] {
        let container = format!("{name} {{\n}}");

        for actions in [format!("\n{container}\n"), MIXED.replace("        switch \"deep\"", &format!("        {container}\n        switch \"deep\""))] {
            let found = errors(&content(&actions));

            assert_eq!(found.len(), 1, "{found:?}");
            assert_eq!(found[0].0, format!("unexpected node `{name}`"));
            assert!(found[0].1.starts_with(&container), "{found:?}");
        }
    }
}

#[test]
fn fields_still_need_their_own_level() {
    let found = errors("enabled true\nfile \"*\"\nmaker \"m\" {\n    make \"x\"\n}\nactions {\n    file \"*\"\n}\n");

    assert_eq!(found.into_iter().map(|(message, _)| message).collect::<Vec<_>>(), vec!["unexpected node `maker`", "unexpected node `file`"]);
}

#[test]
fn errors_point_at_the_right_action() {
    let broken = [
        ("from r\"old (\\w+)\"", "from r\"old (\\w+\"", "replacer #1", "("),
        ("prepare r\"(\\w+)\"", "prepare r\"[\"", "second", "["),
        ("match r\"^(\\w+)$\"", "match r\"^(\\w+$\"", "names", "("),
        ("build \"#[deep]($[0])\"", "build \"#[gone]($[0])\"", "names/inner", "#[gone]"),
        ("to \"#[order]($[0]) #[names]\"", "to \"#[order]($[0]) #[names] #[lost]\"", "replacer #2", "#[lost]"),
        ("index \"@[0]\"\n            values", "index \"$[0]\"\n            values", "replacer #2/order", "$[0]")
    ];

    for (old, new, action, marked) in broken {
        assert!(MIXED.contains(old), "{old}");

        let content = content(&MIXED.replace(old, new));
        let (patch, source) = Patch::parse("", &content).unwrap();
        let found = validate_source(&patch, &source, [], &[]);
        let found = found.iter().find(|found| found.action == action).unwrap_or_else(|| panic!("{action}: {found:?}"));
        let span = found.span.unwrap();

        assert_eq!(&content[span.offset()..span.offset() + span.len()], marked, "{found:?}");
        assert!(content[..span.offset()].ends_with(&new[..new.find(marked).unwrap()]), "{action}");
    }
}

#[test]
fn spans_follow_mixed_actions() {
    let content = content(MIXED);
    let (patch, source) = Patch::parse("", &content).unwrap();
    let spans = &source.spans.actions;
    let text = |span: &miette::SourceSpan| &content[span.offset()..span.offset() + span.len()];

    assert_eq!((spans.builders.len(), spans.replacers.len(), spans.makers.len(), spans.switches.len(), spans.lists.len()), (1, 2, 1, 1, 1));
    assert_eq!(text(&spans.makers[0].fields["name"]), "\"second\"");
    assert_eq!(text(&spans.switches[0].fields["name"]), "\"kind\"");
    assert_eq!(text(&spans.lists[0].fields["name"]), "\"unused\"");
    assert_eq!(text(&spans.builders[0].actions.builders[0].fields["name"]), "\"inner\"");
    assert_eq!(text(&spans.builders[0].actions.switches[0].fields["name"]), "\"deep\"");
    assert_eq!(text(&spans.replacers[1].actions.lists[0].fields["name"]), "\"order\"");

    for (replacer, spans) in patch.actions.replacers.iter().zip(&spans.replacers) {
        assert!(text(&spans.fields["from"]).contains(&replacer.from));
    }
}
