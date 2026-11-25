// Standard Uses
use std::rc::Rc;

// Crate Uses
use crate::operation::{registry, run, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::operation::actions::Action;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::replacer::Replacer;
use codeshaper_core::shaping::patch::trace::Trace;
use codeshaper_core::shaping::patch::validation::{validate_source, DiagnosticKind};


fn replacer(name: &str, from: &str, to: &str) -> String {
    format!("replacer{name} {{\nlocation \"Line\"\nfrom r\"{from}\"\nto \"{to}\"\n}}\n")
}

fn maker(name: &str, make: &str) -> String {
    format!("maker \"{name}\" {{\nmake \"{make}\"\n}}\n")
}

fn content(actions: &str) -> String {
    format!("enabled true\nalias \"lines\"\nfile \"*.txt\"\nactions {{\n{actions}}}\n")
}

fn traced(actions: &str) -> Trace {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

    operation.trace(&registry(), "", &Patch::from_kdl("", &content(actions)).unwrap(), 9).unwrap()
}

fn paths(actions: &str) -> Vec<String> {
    let mut paths: Vec<String> = traced(actions).actions.into_keys().collect();
    paths.sort();

    paths
}

fn found(actions: &str) -> Vec<(DiagnosticKind, String, String, String)> {
    let content = content(actions);
    let (patch, source) = Patch::parse("", &content).unwrap();

    validate_source(&patch, &source, [], &[]).into_iter()
        .map(|found| {
            let span = found.span.unwrap();
            (found.kind, found.action, found.message, content[span.offset()..span.offset() + span.len()].to_string())
        })
        .collect()
}

fn one(kind: DiagnosticKind, action: &str, message: &str, marked: &str) -> Vec<(DiagnosticKind, String, String, String)> {
    vec![(kind, action.to_string(), message.to_string(), marked.to_string())]
}


#[test]
fn names_are_optional() {
    let patch = Patch::from_kdl("", &content(&(replacer(" \"first\"", "a", "b") + &replacer("", "c", "d")))).unwrap();
    let names: Vec<Option<&str>> = patch.actions.replacers.iter().map(|replacer| replacer.name.as_deref()).collect();

    assert_eq!(names, vec![Some("first"), None]);
}

#[test]
fn names_replace_numbers_without_moving_them() {
    let all = |names: [&str; 3]| names.iter().zip(["old", "new", "newer"]).map(|(name, from)| replacer(name, &format!("^{from}"), "newer")).collect::<String>();

    assert_eq!(paths(&all(["", "", ""])), vec!["replacer #1", "replacer #2", "replacer #3"]);
    assert_eq!(paths(&all(["", " \"mid\"", ""])), vec!["mid", "replacer #1", "replacer #3"]);
    assert_eq!(paths(&all([" \"a b\"", " \"mid\"", " \"é\""])), vec!["a b", "mid", "é"]);
}

#[test]
fn names_change_no_result() {
    let plain = replacer("", "old", "new") + &replacer("", "new (\\w+)", "$[0] new");
    let named = replacer(" \"one\"", "old", "new") + &replacer(" \"two\"", "new (\\w+)", "$[0] new");

    let outcome = |actions: &str| {
        let directory = workspace("replace", &[("p.kdl", content(actions))]);
        run(&directory).unwrap();
        std::fs::read_to_string(directory.path().join("target/a.txt")).unwrap()
    };
    assert_eq!(outcome(&plain), "one new\nkeep\n");
    assert_eq!(outcome(&named), outcome(&plain));

    let (plain, named) = (traced(&plain), traced(&named));
    assert_eq!(plain.action("replacer #1"), named.action("one"));
    assert_eq!(plain.action("replacer #2"), named.action("two"));
    assert_eq!(named.action("two").unwrap().matches, 1);
}

#[test]
fn nested_actions_follow_the_name() {
    let nested = replacer(" \"outer\"", "old", "#[inner]").replace("to ", &format!("actions {{\n{}}}\nto ", maker("inner", "x")));

    assert_eq!(traced(&nested).action("outer/inner").unwrap().calls, 1);
    assert_eq!(traced(&nested.replace(" \"outer\"", "")).action("replacer #1/inner").unwrap().calls, 1);
}

#[test]
fn names_are_used_once() {
    let used = |first: &str| format!("The name 'same' is already used at '{first}'");
    let twice = replacer(" \"same\"", "a", "b") + &replacer(" \"same\"", "c", "d");
    assert_eq!(found(&twice), one(DiagnosticKind::DuplicateAction, "same", &used("same"), "\"same\""));

    let others = [
        ("builder \"same\" {\nlocation \"Line\"\nmatch \"a\"\nbuild \"b\"\n}\n".to_string(), "#[same]"),
        (maker("same", "x"), "#[same]"),
        ("switch \"same\" {\nindex \"a\"\ncases a=\"b\"\n}\n".to_string(), "#[same]"),
        ("list \"same\" {\nindex \"0\"\nvalues \"a\"\n}\n".to_string(), "#[same]")
    ];
    for (other, call) in others {
        for actions in [other.clone() + &replacer(" \"same\"", "a", call), replacer(" \"same\"", "a", call) + &other] {
            let found = found(&actions);
            let duplicates: Vec<_> = found.iter().filter(|found| found.0 == DiagnosticKind::DuplicateAction).collect();

            assert_eq!(duplicates.len(), 1, "{actions}\n{found:?}");
            assert_eq!((duplicates[0].2.as_str(), duplicates[0].3.as_str()), (used("same").as_str(), "\"same\""));
        }
    }

    let nested = replacer(" \"same\"", "a", "#[same]").replace("to ", &format!("actions {{\n{}}}\nto ", maker("same", "x")));
    assert_eq!(found(&nested).iter().filter(|found| found.0 == DiagnosticKind::DuplicateAction).count(), 1);
}

#[test]
fn blocks_have_their_own_names() {
    let block = |name: &str| format!("patch \"{name}\" {{\n{}}}\n", content(&replacer(" \"same\"", "a", "b")));
    let patches = Patch::parse_all("", &(block("one") + &block("two"))).unwrap();

    assert_eq!(patches.len(), 2);
    for named in &patches {
        assert!(validate_source(&named.patch, &named.source, [], &[]).is_empty());
    }
}

#[test]
fn replacers_cannot_be_called() {
    let message = "'other' is a replacer, replacers can't be called";
    let called = replacer(" \"other\"", "a", "b") + &replacer("", "c", "x #[other]");
    assert_eq!(found(&called), one(DiagnosticKind::ActionNotFound, "replacer #2", message, "#[other]"));

    let itself = replacer(" \"other\"", "a", "#[other]");
    assert_eq!(found(&itself), one(DiagnosticKind::ActionNotFound, "other", message, "#[other]"));

    let through = replacer(" \"other\"", "a", "#[m]") + &maker("m", "#[other]");
    assert_eq!(found(&through), one(DiagnosticKind::ActionNotFound, "m", message, "#[other]"));

    assert_eq!(found(&replacer("", "a", "#[other]"))[0].2, "No action named 'other'");
}

#[test]
fn replacers_are_never_found_by_name() {
    let patch = Patch::from_kdl("", &content(&(replacer(" \"same\"", "a", "b") + &maker("m", "x")))).unwrap();
    let controller = PatchController::with(Rc::new(patch));

    assert!(controller.find_action("same", &controller).is_none());
    assert!(controller.find_action("", &controller).is_none());
    assert!(controller.find_action("m", &controller).is_some());
    assert!(controller.actions.iter().any(|action| action.get().name() == "same"));
}

#[test]
fn named_replacers_are_not_unused() {
    assert_eq!(found(&replacer(" \"lonely\"", "a", "b")), vec![]);
    assert_eq!(found(&(replacer(" \"lonely\"", "a", "b") + &maker("m", "x"))).len(), 1);
}

#[test]
fn names_fit_in_paths() {
    let characters = "A replacer's name can't have '/', '#' or control characters";

    for (name, message) in [("", "The replacer's name is empty"), ("  ", "The replacer's name is empty"), ("a/b", characters), ("replacer #2", characters), ("a\\nb", characters)] {
        let written = format!("\"{name}\"");
        let actions = replacer("", "a", "b") + &replacer(&format!(" {written}"), "a", "b");

        assert_eq!(found(&actions), one(DiagnosticKind::IncorrectName, "replacer #2", message, &written));
    }

    assert_eq!(DiagnosticKind::IncorrectName.code(), "incorrect-name");
    assert_eq!(found(&replacer(" \"a-b_c.d é 日\"", "a", "b")), vec![]);
    assert_eq!((Replacer::label(Some("x"), 4), Replacer::label(Some("#"), 4), Replacer::label(None, 4)), ("x".to_string(), "replacer #5".to_string(), "replacer #5".to_string()));
}

#[test]
fn names_are_one_text() {
    for written in [" 1", " true", " \"a\" \"b\"", " name=\"a\""] {
        assert!(Patch::from_kdl("", &content(&replacer(written, "a", "b"))).is_err(), "{written}");
    }
}

#[test]
fn errors_carry_the_name() {
    let broken = replacer(" \"shown\"", "(", "#[gone]");
    let found = found(&broken);

    assert_eq!(found.iter().map(|found| (found.1.as_str(), found.3.as_str())).collect::<Vec<_>>(), vec![("shown", "("), ("shown", "#[gone]")]);
}
