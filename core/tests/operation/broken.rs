// Standard Uses

// Crate Uses
use crate::operation::{patch, registry, rename, workspace};

// External Uses
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::patch::Patch;
use miette::Result;


fn preview(content: &str) -> Result<String> {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("broken.kdl", content).unwrap();

    operation.preview(&registry(), "broken.kdl", &patch)?.remove(0).result
}

fn with_actions(to: &str, actions: &str) -> String {
    patch("lines", "Line", r"old (\w+)", to).replace("    replacers {", &format!("{actions}\n    replacers {{"))
}


#[test]
fn unset_groups_are_empty_text() {
    let content = rename("lines", "Line").replace(r"old (\w+)", r"(gone)?old (\w+)").replace("new $[0]", "[$[0]|$[1]]");

    assert_eq!(preview(&content).unwrap(), "[|one]\nkeep\n");
}

#[test]
fn inputs_that_do_not_match_are_not_collected() {
    let content = with_actions("new #[names]", r#"    builders {
        builder "names" {
            location "Line"
            match r"old (\w+)"
            build "<$[0]>"
            collect true
        }
    }"#);

    assert_eq!(preview(&content).unwrap(), "new <one>\nkeep\n");
}

#[test]
fn broken_patches_are_refused() {
    let missing = rename("lines", "Line").replace("$[0]", "$[5]");
    let regex = rename("lines", "Line").replace(r"old (\w+)", r"old (\w+");

    for content in [missing, regex] {
        assert!(preview(&content).is_err());
    }
}

#[test]
fn broken_patches_are_not_traced() {
    let directory = workspace("replace", &[]);
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let patch = Patch::from_kdl("broken.kdl", &rename("lines", "Line").replace("$[0]", "$[5]")).unwrap();

    assert!(operation.trace(&registry(), "broken.kdl", &patch, 3).is_err());
}

#[test]
fn switches_without_a_matching_case_use_the_default() {
    let content = with_actions("new #[kind]", r#"    switchers {
        switch "kind" {
            cases a="x"
            index "z"
            default "other"
        }
    }"#);

    assert_eq!(preview(&content).unwrap(), "new other\nkeep\n");
}

fn getter(to: &str) -> String {
    with_actions(to, r#"    makers {
        maker "getter" {
            prepare r"(\w+) (\w+)"
            make "$[0] get_$[1]()"
        }
    }"#)
}

#[test]
fn prepare_runs_on_the_first_argument() {
    let one = getter("#[getter]($[0])").replace(r"old (\w+)", r"(old \w+)");
    assert_eq!(preview(&one).unwrap(), "old get_one()\nkeep\n");

    let twice = getter("#[getter]($[0]) #[getter](\\\"a b\\\")").replace(r"old (\w+)", r"(old \w+)");
    assert_eq!(preview(&twice).unwrap(), "old get_one() a get_b()\nkeep\n");
}

#[test]
fn arguments_prepare_does_not_match_leave_the_text_alone() {
    assert_eq!(preview(&getter("#[getter]($[0])")).unwrap(), "old one\nkeep\n");
}

#[test]
fn prepared_makers_need_an_argument() {
    assert!(preview(&getter("#[getter]")).unwrap_err().to_string().contains("is called without arguments"));
}

#[test]
fn recursive_actions_are_refused() {
    let content = with_actions("new #[a]", r##"    makers {
        maker "a" {
            make "#[b]"
        }
        maker "b" {
            make "#[a]"
        }
    }"##);

    assert!(preview(&content).unwrap_err().to_string().contains("calls back to this action"));
}

#[test]
fn lists_past_the_end_use_the_default() {
    let content = with_actions("new #[order]", r#"    lists {
        list "order" {
            values "x"
            index "9"
            default "other"
        }
    }"#);

    assert_eq!(preview(&content).unwrap(), "new other\nkeep\n");
}

#[test]
fn named_groups_are_locals() {
    let content = rename("lines", "Line").replace(r"old (\w+)", r"(?P<verb>old) (?P<what>\w+)").replace("new $[0]", "$[what] $[0] $[verb]");

    assert_eq!(preview(&content).unwrap(), "one old old\nkeep\n");
    assert!(preview(&content.replace("$[what]", "$[gone]")).unwrap_err().to_string().contains("'$[gone]' is not a capture group"));
}
