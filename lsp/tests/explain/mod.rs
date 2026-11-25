// Standard Uses
use std::collections::BTreeSet;
use std::rc::Rc;

// Crate Uses
use crate::project;

// External Uses
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::controller::PatchController;
use codeshaper::core::shaping::patch::file::BLOCK;
use codeshaper::core::shaping::patch::pattern::FLAGS;
use codeshaper::core::shaping::project::settings;
use codeshaper_lsp::completion::{completion, document};
use codeshaper_lsp::explain::{self, explained, Explained, CONFIGURATION, PATCH, REQUIRED, SETTINGS};
use codeshaper_lsp::hover::hover;
use codeshaper_lsp::schema;
use codeshaper_lsp::syntax::Outline;
use codeshaper_lsp::trace::Traces;
use lsp_types::Documentation;


const FULL: &str = r##"enabled true
alias "cpp"
file "*.cpp"
actions {
    builder "b" {
        location "identifier"
        reference-location "identifier"
        match "x"
        flags "i"
        build "x"
        collect true
        separator ","
        actions { }
    }
    replacer "r" {
        location "identifier"
        from "x"
        to "x"
        flags "i"
        reference-location "identifier"
        actions { }
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
        index "0"
        default "x"
    }
}
"##;

const PROJECT: &str = "name \"p\"\ntarget \"text\" {\n    files \"*\"\n    exclude \"b\"\n}\nast-set \"cpp\"\ndescription \"d\"\nconstant \"c\" \"d\"\nvariable \"v\" default=\"d\"\n";
const OPERATION: &str = "name \"o\"\nproject \"p\"\ntarget \"t\" {\n    files \"*\"\n    exclude \"b\"\n}\noutput \"o\"\nbackup \"b\"\nresult \"replace\"\nvariables v=\"d\"\n";

type Decodes = fn(&str) -> bool;

fn documents() -> [(&'static [Explained], &'static str, Decodes); 3] {
    [
        (PATCH, FULL, |text| Patch::from_kdl("", text).is_ok()),
        (SETTINGS, PROJECT, |text| settings::from_kdl(text).is_ok()),
        (CONFIGURATION, OPERATION, |text| configuration::from_kdl(text).is_ok())
    ]
}

fn whole(text: &str, parent: Option<&str>, name: &str) -> Vec<std::ops::Range<usize>> {
    let outline = Outline::parse(text);

    outline.nodes.iter()
        .filter(|node| node.name == name && node.parent.map(|parent| outline.nodes[parent].name.as_str()) == parent)
        .map(|node| {
            let line = text[node.name_range.start..].find('\n').map(|length| node.name_range.start + length).unwrap_or(text.len());
            let block = node.children.as_ref().map(|children| children.end + 1).unwrap_or(line);

            node.name_range.start..line.max(block)
        })
        .collect()
}

fn replaced(text: &str, explained: &Explained, with: &str) -> String {
    let mut text = text.to_string();
    let found = whole(&text, explained.parent, explained.name);
    assert!(!found.is_empty(), "{explained:?}");

    for range in found.into_iter().rev() {
        text.replace_range(range, with);
    }

    text
}

fn applied(actions: &str, source: &str) -> String {
    let patch = Patch::from_kdl("", &format!("enabled true\nfile \"*\"\nactions {{\n{actions}\n}}\n")).unwrap();
    let registry = codeshaper::registry();

    PatchController::with(Rc::new(patch)).apply(registry.backend("cpp").unwrap(), source).unwrap()
}

fn replacer(location: &str, from: &str, to: &str) -> String {
    format!("replacer {{\nlocation \"{location}\"\nfrom r\"{from}\"\nto \"{to}\"\n}}\n")
}

fn builder(location: &str, pattern: &str, build: &str, more: &str) -> String {
    format!("builder \"b\" {{\nlocation \"{location}\"\nmatch r\"{pattern}\"\nbuild \"{build}\"\n{more}\n}}\n")
}


#[test]
fn every_name_is_explained_once() {
    let mut expected: BTreeSet<(Option<&str>, &str)> = BTreeSet::from([(None, BLOCK)]);
    for parent in [None, Some("actions"), Some("builder"), Some("replacer"), Some("maker"), Some("switch"), Some("list")] {
        expected.extend(schema::children(parent).iter().map(|name| (parent, *name)));
    }

    assert_eq!(PATCH.iter().map(|explained| (explained.parent, explained.name)).collect::<BTreeSet<_>>(), expected);
    for (table, _, _) in documents() {
        assert_eq!(table.iter().map(|explained| (explained.parent, explained.name)).collect::<BTreeSet<_>>().len(), table.len());
    }

    for (table, text, names) in [(SETTINGS, PROJECT, 8), (CONFIGURATION, OPERATION, 9)] {
        let outline = Outline::parse(text);
        assert_eq!((outline.nodes.len(), table.len()), (names, names));

        for node in &outline.nodes {
            assert!(explained(table, node.parent.map(|parent| outline.nodes[parent].name.as_str()), &node.name).is_some(), "{}", node.name);
        }
    }
}

#[test]
fn required_names_are_required() {
    for (table, text, decodes) in documents() {
        assert!(decodes(text), "{text}");

        for explained in table.iter().filter(|explained| explained.name != BLOCK && explained.need.is_some()) {
            let without = replaced(text, explained, "");

            assert_eq!(decodes(&without), explained.need != Some(REQUIRED), "{:?} {}\n{without}", explained.parent, explained.name);
        }
    }
}

#[test]
fn defaults_are_those_of_the_decoder() {
    let text = "file \"*\"\nactions {\n    builder \"b\" {\n        location \"a\"\n        match \"a\"\n        build \"a\"\n    }\n    \
        replacer {\n        location \"a\"\n        from \"a\"\n        to \"a\"\n    }\n    maker \"m\" {\n        make \"a\"\n    }\n}\n";
    let patch = Patch::from_kdl("", text).unwrap();
    let (builder, replacer, maker) = (&patch.actions.builders[0], &patch.actions.replacers[0], &patch.actions.makers[0]);

    assert_eq!((patch.enabled, patch.alias.as_str(), builder.collect, builder.separator.as_str()), (true, "", false, ""));
    assert_eq!((builder.flags.as_str(), replacer.flags.as_str(), maker.flags.as_str()), ("", "", ""));
    assert_eq!((builder.reference_location.as_ref(), replacer.target_location(), replacer.name.as_ref()), (None, "a", None));
}

#[test]
fn examples_decode_in_their_place() {
    for (table, text, decodes) in documents() {
        for explained in table.iter().filter(|explained| explained.name != BLOCK && explained.parent != Some("target")) {
            let alone = match explained.name {
                "reference-location" => replaced(text, &Explained { name: "location", ..*explained }, ""),
                "collect" => ["location", "reference-location"].iter()
                    .fold(text.to_string(), |text, name| replaced(&text, &Explained { name, ..*explained }, "")),
                _ => text.to_string()
            };
            let shown = replaced(&alone, explained, explained.example);

            assert!(decodes(&shown), "{shown}");
            assert!(Outline::parse(explained.example).nodes.iter().any(|node| node.name == explained.name));
        }

        for explained in table.iter().filter(|explained| explained.parent == Some("target")) {
            let target = Explained { parent: None, name: "target", ..*explained };
            assert!(decodes(&replaced(text, &target, explained.example)), "{}", explained.example);
        }
    }

    let block = explained(PATCH, None, BLOCK).unwrap().example;
    assert_eq!(Patch::parse_all("", block).unwrap()[0].name.as_deref(), Some("rename"));
}

#[test]
fn examples_use_known_names_and_locations() {
    let registry = codeshaper::registry();
    let backend = registry.backend("cpp").unwrap();

    for explained in PATCH {
        let outline = Outline::parse(explained.example);

        for (index, node) in outline.nodes.iter().enumerate() {
            let parent = node.parent.map(|parent| outline.nodes[parent].name.as_str()).or(explained.parent).filter(|parent| *parent != BLOCK);
            assert!(explain::explained(PATCH, parent, &node.name).is_some(), "{} under {parent:?}", node.name);

            if let Some(location) = outline.field(index, "location") { backend.check_location(&location.value).unwrap(); }
        }
    }
}

#[test]
fn flags_are_all_named() {
    for parent in ["builder", "replacer", "maker"] {
        let text = explained(PATCH, Some(parent), "flags").unwrap().text;

        assert!(text.starts_with(&format!("Regex flags for `{}`: ", schema::regex_field(parent).unwrap())));
        for (flag, meaning) in FLAGS {
            assert!(text.contains(&format!("`{flag}` ({meaning})")), "{text}");
        }
    }
}

#[test]
fn builders_read_around_or_inside() {
    let around = builder("function_definition", r"void (\w+)", "$[0]", "") + &replacer("function_definition.body", "x", "#[b]");
    assert_eq!(applied(&around, "void one() { x; }\nvoid two() { x; }\n"), "void one() { one; }\nvoid two() { two; }\n");

    let inside = builder("identifier", r"(\w+)", "$[0]", "") + &replacer("function_definition.body", r"\}", "#[b]}");
    assert_eq!(applied(&inside, "void f() { a; b; }\n"), "void f() { a; b; a}\n");

    let referring = |location: &str, reference: &str| {
        builder(location, r"(\w+)", "$[0]", &format!("reference-location \"{reference}\"")) + &replacer("function_definition.body", "x", "#[b]")
    };
    let source = "class A { void f() { x; } void g() {} };\n";
    assert_eq!(applied(&referring("class_specifier", "class_specifier.name"), source), source.replace('x', "A"));
    assert_eq!(applied(&referring("function_definition.body", "class_specifier"), source), source.replace('x', "class"));
    assert_eq!(applied(&referring("enum_specifier", "class_specifier.name"), source), source);
    assert_eq!(applied(&referring("class_specifier", "enum_specifier"), source), source);

    let first = builder("function_definition", r"(\d)", "$[0]", "") + &replacer("function_definition.body", "x", "#[b]");
    assert_eq!(applied(&first, "void f() { x; 1; 2; }\n"), "void f() { 1; 1; 2; }\n");
}

#[test]
fn builders_collect_every_place() {
    let collected = |more: &str| builder("function_definition", r"void (\w+)", "$[0]", more) + &replacer("return_statement", "x", "#[b]");
    let source = "void one() { return x; }\nint no() {}\nvoid two() {}\n";

    assert_eq!(applied(&collected("collect true\nseparator \"+\""), source), source.replace('x', "one+two"));
    assert_eq!(applied(&collected("collect true"), source), source.replace('x', "onetwo"));
    assert_eq!(applied(&collected("separator \"+\""), source), source.replace('x', "one"));

    let inside = builder("class_specifier", r"(\w+)", "$[0]", "reference-location \"field_identifier\"\ncollect true\nseparator \"+\"")
        + &replacer("return_statement", "x", "#[b]");
    let source = "class A { int f() { return x; } int g() {} };\nclass B { int h() { return x; } };\nint i() { return x; }\n";
    assert_eq!(applied(&inside, source), source.replacen('x', "f+g", 1).replacen('x', "h", 1));
}

#[test]
fn nothing_given_skips_the_place() {
    let source = "void f() { x; x; }\n";
    let body = |actions: &str, to: &str| applied(&(actions.to_string() + &replacer("function_definition.body", "x", to)), source);

    assert_eq!(body(&builder("function_definition", r"nope (\w+)", "$[0]", ""), "#[b]"), source);
    assert_eq!(body(&builder("function_definition", r"nope (\w+)", "y", ""), "#[b]"), source.replace('x', "y"));

    let switch = |more: &str| format!("switch \"s\" {{\ncases a=\"1\"\nindex \"b\"\n{more}\n}}\n");
    assert_eq!(body(&switch(""), "#[s]"), source);
    assert_eq!(body(&switch("default \"2\""), "#[s]"), source.replace('x', "2"));

    let list = |index: &str, more: &str| format!("list \"l\" {{\nvalues \"a\" \"b\"\n{index}\n{more}\n}}\n");
    assert_eq!(body(&list("index \"0\"", ""), "#[l]"), source.replace('x', "a"));
    assert_eq!(body(&list("index \"1\"", ""), "#[l]"), source.replace('x', "b"));
    assert_eq!(body(&list("index \"2\"", ""), "#[l]"), source);
    assert_eq!(body(&list("index \"2\"", "default \"c\""), "#[l]"), source.replace('x', "c"));
    assert_eq!(body(&list("", "default \"c\""), "#[l]"), source);

    let some = replacer("function_definition.body", r"(x|y)", "#[s]($[0])") + "switch \"s\" {\ncases x=\"1\"\nindex \"@[0]\"\n}\n";
    assert_eq!(applied(&some, "void f() { x; y; }\nvoid g() { x; }\n"), "void f() { x; y; }\nvoid g() { 1; }\n");
}

#[test]
fn replacers_run_in_order_on_their_text() {
    assert_eq!(applied(&(replacer("identifier", "a", "b") + &replacer("identifier", "b", "c")), "int a;\n"), "int c;\n");
    assert_eq!(applied(&(replacer("identifier", "b", "c") + &replacer("identifier", "a", "b")), "int a;\n"), "int b;\n");
    assert_eq!(applied(&replacer("identifier", "a", "b"), "int aa;\n"), "int bb;\n");

    let referring = |reference: &str| replacer("function_definition.body", r"f", "g").replace("from", &format!("reference-location \"{reference}\"\nfrom"));
    assert_eq!(applied(&referring("function_definition"), "void f() { f; }\n"), "void g() { g; }\n");
    assert_eq!(applied(&referring("identifier"), "void f() { f; ff; }\n"), "void f() { g; ff; }\n");
}

#[test]
fn nested_actions_are_called_from_anywhere() {
    let nested = replacer("identifier", "^a$", "x").replace("to", "actions {\nmaker \"m\" {\nmake \"<@[0]>\"\n}\n}\nto") + &replacer("identifier", "^(b)$", "#[m]($[0])");

    assert_eq!(applied(&nested, "int a; int b;\n"), "int x; int <b>;\n");
}

#[test]
fn makers_prepare_their_first_argument() {
    let maker = "maker \"m\" {\nprepare r\"(\\w)(\\w)\"\nmake \"$[1]$[0]@[0]@[1]\"\n}\n";

    assert_eq!(applied(&(maker.to_string() + &replacer("identifier", "^(ab)(c)$", "#[m]($[0], $[1])")), "int abc;\n"), "int baabc;\n");
}

#[test]
fn file_patterns_cross_folders() {
    let patch = |file: &str| Patch { file: file.to_string(), ..Patch::default() };

    assert!(patch("src/*.cpp").matches(std::path::Path::new("src/a/b.cpp")));
    assert!(patch("*.cpp").matches(std::path::Path::new("a/b.cpp")));
    assert!(!patch("src/*.cpp").matches(std::path::Path::new("a.cpp")));
}

#[test]
fn target_files_stay_in_their_folder() {
    let directory = tempfile::tempdir().unwrap();
    for file in ["target/a.cpp", "target/in/b.cpp"] {
        let path = directory.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    let (_project, project) = project(&[]);
    let listed = |options: &str| {
        let text = format!("name \"o\"\nproject \"{}\"\ntarget \"target\" {{\n{options}\n}}\noutput \"o\"\nresult \"replace\"\n", project.display().to_string().replace('\\', "/"));
        std::fs::write(directory.path().join("configuration.kdl"), text).unwrap();
        let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();

        operation.target(&codeshaper::registry()).unwrap().files().len()
    };

    assert_eq!((listed(""), listed("files \"*.cpp\""), listed("files \"**/*.cpp\""), listed("exclude \"in/**\"")), (2, 1, 2, 1));
}

#[test]
fn disabled_patches_are_still_checked() {
    let (directory, project) = crate::preview::operation();
    let text = crate::preview::RENAME.replace("from r\"old\"", "from r\"(\"");
    std::fs::write(project.join("patches/rename.kdl"), &text).unwrap();
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let registry = codeshaper::registry();

    assert!(text.contains("enabled false"));
    assert_eq!(operation.check(&registry).unwrap().len(), 1);
    assert!(operation.session(&registry).unwrap().is_empty());
}

fn hovered(text: &str, offset: usize) -> Option<(String, std::ops::Range<usize>)> {
    let (_directory, project) = project(&[]);

    hover(&codeshaper::registry(), &project, &mut Traces::default(), text, offset)
}

#[test]
fn patch_names_are_explained() {
    let outline = Outline::parse(FULL);

    for node in &outline.nodes {
        let parent = node.parent.map(|parent| outline.nodes[parent].name.as_str());
        let expected = explained(PATCH, parent, &node.name).unwrap().markdown();

        for offset in [node.name_range.start, node.name_range.end] {
            assert_eq!(hovered(FULL, offset), Some((expected.clone(), node.name_range.clone())), "{}", node.name);
        }
    }

    assert_eq!(hovered(FULL, FULL.find("separator").unwrap()).unwrap().0, "**separator** · optional, nothing if left out\n\n\
        What goes between the texts `collect` joins. Not used without `collect`.\n\n```kdl\nseparator \", \"\n```");
    assert!(hovered(FULL, FULL.find("builder").unwrap()).unwrap().0.starts_with("**builder**\n\nMakes text out of"));
}

#[test]
fn values_keep_their_hovers() {
    let at = |needle: &str| hovered(FULL, FULL.find(needle).unwrap() + 1).map(|(value, _)| value);

    assert!(at("\"cpp\"").unwrap().starts_with("Language backend `cpp`"));
    assert!(at("\"identifier\"").unwrap().starts_with("cpp: node kind"));
    assert!(at("\"b\"").unwrap().starts_with("Called from"));
    assert_eq!(at("true\n"), None);
}

#[test]
fn blocks_and_nested_actions_are_explained() {
    let text = format!("patch \"é\" {{\n{FULL}}}\npatch \"two\" {{\n    actions {{\n        builder \"b\" {{\n            actions {{\n                maker \"m\" {{\n                    ma");
    let at = |needle: &str| hovered(&text, text.rfind(needle).unwrap() + 1).map(|(value, range)| (value.lines().next().unwrap().to_string(), text[range].to_string()));
    let pair = |first: &str, marked: &str| Some((first.to_string(), marked.to_string()));

    assert_eq!(at("patch"), pair("**patch** · optional", "patch"));
    assert_eq!(at("collect"), pair("**collect** · optional, `false` if left out", "collect"));
    assert_eq!(at("actions"), pair("**actions** · optional", "actions"));
    assert_eq!(at("maker"), pair("**maker**", "maker"));
    assert_eq!(at("ma"), None);
    assert_eq!(at("enabled"), pair("**enabled** · optional, `true` if left out", "enabled"));
}

#[test]
fn unknown_and_commented_names_say_nothing() {
    let text = "nothing 1\n/-maker \"m\" {\n    make \"x\"\n}\nactions {\n    builders {\n        builder \"b\" {\n        }\n    }\n    /-maker \"n\" {\n        make \"x\"\n    }\n    location \"x\"\n}\n";
    let outline = Outline::parse(text);
    let found: Vec<&str> = outline.nodes.iter()
        .filter(|node| explain::hover(PATCH, &outline, node.name_range.start).is_some())
        .map(|node| node.name.as_str())
        .collect();

    assert_eq!(found, vec!["actions"]);
}

#[test]
fn other_documents_are_explained() {
    let (directory, project) = crate::preview::operation();
    let settings = project.join("project.kdl");
    let configuration = directory.path().join("configuration.kdl");
    let first = |path: &std::path::Path, text: &str, needle: &str| {
        explain::document(path, text, text.find(needle).unwrap() + 1).map(|(value, range)| (value.lines().next().unwrap().to_string(), text[range].to_string()))
    };
    let pair = |first: &str, marked: &str| Some((first.to_string(), marked.to_string()));

    assert_eq!(first(&settings, PROJECT, "target"), pair("**target** · required", "target"));
    assert_eq!(first(&settings, PROJECT, "ast-set"), pair("**ast-set** · required", "ast-set"));
    assert_eq!(first(&settings, PROJECT, "files"), pair("**files** · optional, every file if left out", "files"));
    assert_eq!(first(&settings, &PROJECT.replace("\"text\"", "\"godot\""), "files"), None);
    assert_eq!(first(&settings, PROJECT, "\"cpp\""), None);
    assert_eq!(first(&configuration, OPERATION, "result"), pair("**result** · required", "result"));
    assert_eq!(first(&configuration, OPERATION, "backup"), pair("**backup** · optional, needed by \"backup-and-replace\"", "backup"));

    let shown = OPERATION.replace("project \"p\"", "project \"shaping_project\"");
    assert_eq!(first(&configuration, &shown, "exclude"), pair("**exclude** · optional, the project's if left out", "exclude"));
    assert_eq!(first(&directory.path().join("notes.txt"), PROJECT, "target"), None);
}

#[test]
fn suggestions_are_documented() {
    let (_directory, project) = project(&[]);
    let registry = codeshaper::registry();
    let shown = |text: &str, offset: usize| -> Vec<(String, bool)> {
        completion(&registry, &project, text, offset).into_iter()
            .map(|item| (item.label, matches!(item.documentation, Some(Documentation::MarkupContent(_)))))
            .collect()
    };

    for text in ["", "enabled true\n", "actions {\n    ", "actions {\n    builder \"b\" {\n        ", "patch \"p\" {\n    ", "actions {\n    switch \"s\" {\n        "] {
        let found = shown(text, text.len());
        assert!(!found.is_empty() && found.iter().all(|(_, documented)| *documented), "{text}: {found:?}");
    }

    let text = "actions {\n    maker \"m\" {\n        ";
    let found = completion(&registry, &project, text, text.len());
    let Some(Documentation::MarkupContent(content)) = &found[0].documentation else { panic!("expected documentation") };
    assert_eq!((found[0].label.as_str(), content.value.as_str()), ("prepare", explained(PATCH, Some("maker"), "prepare").unwrap().markdown().as_str()));

    let text = "name \"p\"\ntarget \"text\" {\n    \n}\nast-set \"cpp\"\ndescription \"\"\n";
    let options = document(&registry, &project.join("project.kdl"), text, text.find("    \n").unwrap() + 4);
    assert_eq!(options.iter().map(|item| (item.label.as_str(), item.documentation.is_some())).collect::<Vec<_>>(), vec![("files", true), ("exclude", true)]);
}
