// Standard Uses

// Crate Uses

// External Uses
use codeshaper_lsp::syntax::{Context, Outline};


const PATCH: &str = r##"enabled true
file "*.cpp" // comment
actions {
    builder "names" {
        match r#"(\w+) "quoted""#
        build "$[0]"
    }
}
"##;


#[test]
fn outline_follows_nesting_and_strings() {
    let outline = Outline::parse(PATCH);
    let names: Vec<&str> = outline.nodes.iter().map(|node| node.name.as_str()).collect();

    assert_eq!(names, vec!["enabled", "file", "actions", "builder", "match", "build"]);
    assert_eq!(outline.argument("file"), Some("*.cpp"));

    let builder = &outline.nodes[3];
    assert_eq!(outline.nodes[builder.parent.unwrap()].name, "actions");
    assert_eq!(builder.arguments[0].value, "names");

    let regex = &outline.nodes[4].arguments[0];
    assert_eq!(regex.value, r#"(\w+) "quoted""#);
    assert_eq!(&PATCH[regex.range.clone()], r##"r#"(\w+) "quoted""#"##);
    assert_eq!(outline.actions().map(|(_, name)| name.value.as_str()).collect::<Vec<_>>(), vec!["names"]);
}

#[test]
fn half_typed_text_keeps_its_context() {
    let text = "actions {\n    builder \"names\" {\n        loc\n        build \"#[na";
    let outline = Outline::parse(text);

    let at_name = text.find("loc").unwrap() + 3;
    let Context::Name { parent, prefix } = outline.context(text, at_name) else { panic!("expected a name") };
    assert_eq!(outline.nodes[parent.unwrap()].name, "builder");
    assert_eq!(&text[prefix], "loc");

    let Context::Value { node, before, .. } = outline.context(text, text.len()) else { panic!("expected a value") };
    assert_eq!(outline.nodes[node].name, "build");
    assert_eq!(before, "#[na");
}

#[test]
fn values_after_a_name_are_not_names() {
    let text = "enabled true\n";

    assert_eq!(Outline::parse(text).context(text, text.find("true").unwrap() + 2), Context::None);
}

#[test]
fn paths_follow_mixed_actions() {
    let text = "actions {\n    replacer {\n        actions {\n            replacer {\n            }\n            maker \"m\" {\n            }\n        }\n    }\n    \
        maker \"n\" {\n    }\n    replacer {\n        actions {\n            list \"l\" {\n            }\n        }\n    }\n}\n";
    let outline = Outline::parse(text);
    let paths: Vec<String> = (0..outline.nodes.len())
        .filter(|index| outline.nodes[*index].name != "actions")
        .filter_map(|index| outline.action_path(index))
        .collect();

    assert_eq!(paths, vec!["replacer #1", "replacer #1/replacer #1", "replacer #1/m", "n", "replacer #2", "replacer #2/l"]);
}
