// Standard Uses

// Crate Uses
use crate::project;

// External Uses
use codeshaper::core::shaping::patch::Patch;
use codeshaper_lsp::completion::completion;
use codeshaper_lsp::schema;
use codeshaper_lsp::syntax::Outline;
use lsp_types::{CompletionTextEdit, Position};


fn labels(text: &str, offset: usize) -> Vec<String> {
    let (_directory, project) = project(&[]);

    completion(&codeshaper::registry(), &project, text, offset).into_iter().map(|item| item.label).collect()
}


#[test]
fn schema_matches_the_models() {
    let text = r##"enabled true
alias ""
file "*.cpp"
actions {
    builder "b" {
        location "Identifier"
        reference-location "Identifier"
        match "x"
        flags "i"
        build "x"
        collect true
        separator ","
        actions {
        }
    }
    replacer {
        location "Identifier"
        from "x"
        to "x"
        flags "g"
        reference-location "Identifier"
        reference "x"
        actions {
        }
    }
    maker "m" {
        prepare "x"
        flags "i"
        make "x"
    }
    switch "s" {
        cases a="b"
        index "x"
        default "x"
    }
    list "l" {
        values "a" "b"
        index "x"
        default "x"
    }
}
"##;

    Patch::from_kdl("schema.kdl", text).unwrap();

    let outline = Outline::parse(text);
    let used: Vec<(Option<&str>, &str)> = outline.nodes.iter()
        .map(|node| (node.parent.map(|parent| outline.nodes[parent].name.as_str()), node.name.as_str()))
        .collect();

    for (parent, name) in &used {
        assert!(schema::children(*parent).contains(name), "{name} under {parent:?} is not in the schema");
    }

    for parent in std::iter::once(None).chain(used.iter().map(|(_, name)| Some(*name))) {
        for name in schema::children(parent) {
            assert!(used.contains(&(parent, *name)), "{name} under {parent:?} is not in the test patch");
        }
    }
}

#[test]
fn field_names_follow_the_enclosing_node() {
    let text = "actions {\n    maker \"m\" {\n        \n    }\n}\n";
    let offset = text.find("        \n").unwrap() + 8;

    assert_eq!(labels(text, offset), vec!["prepare", "flags", "make"]);
    assert_eq!(labels("", 0), vec!["enabled", "alias", "file", "actions", "patch"]);
    assert_eq!(labels("enabled true\n", 13), vec!["enabled", "alias", "file", "actions"]);
}

#[test]
fn actions_offer_every_type() {
    let types = vec!["builder", "replacer", "maker", "switch", "list"];
    let place = |text: &str| labels(&text.replace('|', ""), text.find('|').unwrap());

    assert_eq!(place("actions {\n    |\n}\n"), types);
    assert_eq!(place("actions {\n    maker \"m\" {\n    }\n    bui|\n    replacer {\n    }\n}\n"), types);
    assert_eq!(place("actions {\n    builder \"b\" {\n        actions {\n            |"), types);
    assert_eq!(place("actions {\n    switch \"s\" {\n        |\n    }\n}\n"), vec!["cases", "index", "default"]);
    assert_eq!(place("actions {\n    builders {\n        |\n    }\n}\n"), Vec::<String>::new());
}

#[test]
fn locations_come_from_the_backend() {
    let text = "alias \"\"\nactions {\n    replacer {\n        location \"Function";
    let kinds = labels(text, text.len());

    assert!(kinds.contains(&"FunctionDefinition".to_string()));
    assert!(!kinds.contains(&"Body".to_string()));

    let fields = labels(&format!("{text}Definition."), text.len() + "Definition.".len());
    assert!(fields.contains(&"Body".to_string()));
}

#[test]
fn action_names_inside_expressions() {
    let text = "actions {\n    maker \"pair\" {\n        make \"x\"\n    }\n    replacer {\n        to \"#[p";

    assert_eq!(labels(text, text.len()), vec!["pair"]);
    assert_eq!(labels(&format!("{text}air] "), text.len() + 5), Vec::<String>::new());
}

#[test]
fn locals_offer_the_regex_groups() {
    let text = "actions {\n    replacer {\n        from r\"(\\w+) (?:x) (?P<n>\\d)\"\n        to \"a $[\"\n    }\n}\n";
    let offset = text.find("$[").unwrap() + 2;
    let (_directory, project) = project(&[]);

    let items = completion(&codeshaper::registry(), &project, text, offset);
    let found: Vec<(&str, Option<&str>)> = items.iter().map(|item| (item.label.as_str(), item.detail.as_deref())).collect();

    assert_eq!(found, vec![("0", Some(r"(\w+)")), ("n", Some(r"(?P<n>\d)")), ("1", Some(r"(?P<n>\d)"))]);
    assert_eq!(labels(&text.replace("from", "location"), offset), Vec::<String>::new());
}

#[test]
fn file_offers_target_paths() {
    let (_directory, project) = crate::preview::operation();
    let text = "alias \"\"\nfile \"a.c\"\n";

    let items = completion(&codeshaper::registry(), &project, text, text.find("a.c").unwrap() + 3);

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), vec!["a.cpp", "b.cpp"]);
    let Some(CompletionTextEdit::Edit(edit)) = &items[1].text_edit else { panic!("expected an edit") };
    assert_eq!((edit.range.start, edit.range.end), (Position::new(1, 6), Position::new(1, 9)));
}
