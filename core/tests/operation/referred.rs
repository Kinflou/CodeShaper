// Standard Uses
use std::fs;

// Crate Uses
use crate::operation::{registry, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::trace::Trace;


fn traced(source: &str, actions: &str) -> Trace {
    let directory = workspace("replace", &[]);
    fs::write(directory.path().join("target/a.txt"), source).unwrap();
    fs::write(directory.path().join("target/nested/b.txt"), "").unwrap();

    let patch = Patch::from_kdl("", &format!("enabled true\nfile \"*.txt\"\nactions {{\n{actions}}}\n")).unwrap();
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

    operation.trace(&registry(), "", &patch, 9).unwrap()
}

fn builder(location: &str, reference: &str, more: &str) -> String {
    format!("builder \"b\" {{\nlocation \"{location}\"\nreference-location \"{reference}\"\nmatch r\"(\\w+)\"\nbuild \"$[0]\"\n{more}\n}}\n")
}

fn replacer(location: &str, reference: &str, to: &str) -> String {
    format!("replacer {{\nlocation \"{location}\"\n{reference}\nfrom \"one\"\nto \"{to}\"\n}}\n")
}

fn places(source: &str, trace: &Trace) -> Vec<String> {
    trace.action("b").unwrap().found.iter()
        .map(|found| {
            let range = found.matched.place.clone().unwrap();
            assert_eq!(found.matched.text, source[range.clone()]);

            format!("{}@{}", found.matched.text, range.start)
        })
        .collect()
}


#[test]
fn references_keep_their_places() {
    for source in ["one\ntwo é\nthree\n", "one\r\ntwo é\r\nthree\r\n"] {
        let third = source.find("three").unwrap();
        let second = source.find("two").unwrap();

        let collected = builder("File", "Line", "collect true") + &replacer("Line", "", "#[b]");
        assert_eq!(places(source, &traced(source, &collected)), vec!["one@0".to_string(), format!("two@{second}"), format!("three@{third}")]);

        let first = builder("File", "Line", "") + &replacer("Line", "", "#[b]");
        assert_eq!(places(source, &traced(source, &first)), vec!["one@0"]);
    }
}

#[test]
fn references_are_counted_when_missing() {
    let counted = |actions: &str, action: &str| {
        let trace = traced("one\n", actions);
        trace.action(action).map(|action| (action.inputs, action.unreferred, action.calls))
    };

    for (more, calls) in [("", 0), ("collect true", 1)] {
        assert_eq!(counted(&(builder("Line", "Nothing", more) + &replacer("Line", "", "#[b]")), "b"), Some((0, 1, calls)));
        assert_eq!(counted(&(builder("Nothing", "Line", more) + &replacer("Line", "", "#[b]")), "b"), None);
        assert_eq!(counted(&(builder("Line", "Line", more) + &replacer("Line", "", "#[b]")), "b"), Some((1, 0, 1)));
    }

    assert_eq!(counted(&replacer("Line", "reference-location \"Nothing\"", "x"), "replacer #1"), Some((0, 1, 0)));
    assert_eq!(counted(&replacer("Line", "reference-location \"Line\"", "x"), "replacer #1"), Some((1, 0, 0)));
    assert_eq!(counted(&replacer("Nothing", "reference-location \"Line\"", "x"), "replacer #1"), None);
}
