// Standard Uses
use std::fs;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::{LanguageBackend, Suggestions};
use codeshaper_nav_ast_treesitter::TreeSitterBackend;


const CPP: &str = "\
#include <std>

void MethodOne()
{
    int a = 1;
}

void MethodTwo()
{
    int a = 1;
}
";

fn example(path: &str) -> String {
    fs::read_to_string(format!("../../codeshaper-rs/examples/{path}")).unwrap()
}

fn suggested(source: &str, text: &str) -> Suggestions {
    let backend = TreeSitterBackend::cpp();
    let node = backend.nodes(source).unwrap().iter().position(|node| &source[node.range.clone()] == text).unwrap();

    backend.suggest_locations(source, node).unwrap()
}

fn paths(suggestions: &Suggestions) -> Vec<(&str, usize)> {
    suggestions.paths.iter().map(|path| (path.location.as_str(), path.ranges.len())).collect()
}

fn check(backend: &TreeSitterBackend, source: &str) {
    let nodes = backend.nodes(source).unwrap();

    for (index, node) in nodes.iter().enumerate() {
        assert!(node.parent.is_none_or(|parent| parent < index && nodes[parent].range.start <= node.range.start && node.range.end <= nodes[parent].range.end));

        let found = backend.suggest_locations(source, index).unwrap();
        assert!(node.parent.is_none() || node.broken || !found.paths.is_empty(), "{node:?}");
        assert!(found.paths.len() <= 4);

        for path in &found.paths {
            backend.check_location(&path.location).unwrap();
            assert_eq!(backend.locate(source, &path.location).unwrap(), path.ranges, "{}", path.location);
            assert!(path.ranges.contains(&node.range), "{} {node:?}", path.location);
        }
        if let Some(query) = &found.query {
            backend.check_location(&query.location).unwrap();
            assert_eq!(backend.locate(source, &query.location).unwrap(), vec![node.range.clone()], "{}", query.location);
        }
    }
}

fn accepts_its_names(backend: &TreeSitterBackend, source: &str) {
    for node in backend.nodes(source).unwrap().iter().filter(|node| node.kind != "ERROR") {
        backend.check_location(&node.kind).unwrap();
        if let Some(field) = &node.field { backend.check_location(&format!("{}.{field}", node.kind)).unwrap(); }
    }
}

fn knows_its_names(backend: &TreeSitterBackend) {
    let first = backend.location_names(0);
    let later = backend.location_names(1);
    let names = |known: &[String], more: Vec<String>| known == more.into_iter().collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();

    assert!(names(&first, backend.node_kinds()));
    assert!(names(&later, [backend.node_kinds(), backend.fields()].concat()));

    for name in &first { backend.check_location(name).unwrap(); }
    for name in &later { backend.check_location(&format!("{}.{name}", first[0])).unwrap(); }
}


#[test]
fn suggested_names_are_known() {
    knows_its_names(&TreeSitterBackend::cpp());
    #[cfg(feature = "gdscript")]
    knows_its_names(&TreeSitterBackend::gdscript());
    #[cfg(feature = "godot-resource")]
    knows_its_names(&TreeSitterBackend::godot_resource());
}

#[test]
fn examples_are_suggested_for() {
    let backend = TreeSitterBackend::cpp();

    for name in ["CallerModule.cpp", "Module.cpp", "Module.h", "ReturnModule.cpp"] {
        let source = example(&format!("CPP14/Function Name Correction/target/{name}"));
        check(&backend, &source);
        accepts_its_names(&backend, &source);
    }
}

#[cfg(feature = "gdscript")]
#[test]
fn gdscript_is_suggested_for() {
    let backend = TreeSitterBackend::gdscript();
    let source = example("Godot/Godot 3 to 4 Migration/target/player.gd");

    check(&backend, &source);
    accepts_its_names(&backend, &source);
}

#[cfg(feature = "godot-resource")]
#[test]
fn godot_resources_are_suggested_for() {
    let backend = TreeSitterBackend::godot_resource();

    for name in ["player.tscn", "project.godot"] {
        let source = example(&format!("Godot/Godot 3 to 4 Migration/target/{name}"));
        check(&backend, &source);
        accepts_its_names(&backend, &source);
    }
}

#[test]
fn line_endings_are_kept() {
    check(&TreeSitterBackend::cpp(), &CPP.replace('\n', "\r\n"));
}

#[test]
fn surrounding_names_come_before_other_text() {
    let source = CPP.replacen("int a = 1;", "int a = 7;\n    if (a) {}", 1).replacen("int a = 1;", "int a = 8;\n    if (a) {}", 1);
    let query = suggested(&source, "if (a) {}").query.unwrap().location;

    assert!(query.contains("\"MethodOne\""), "{query}");

    let source = CPP.replacen("int a", "int other", 1);
    let query = suggested(&source, "{\n    int other = 1;\n}").query.unwrap().location;
    assert!(query.contains("\"MethodOne\""), "{query}");
}

#[test]
fn bodies_lead_with_their_field() {
    let body = "{\n    int a = 1;\n}";
    let found = suggested(CPP, body);

    assert_eq!(paths(&found), vec![("function_definition.body", 2), ("compound_statement", 2)]);
    let query = found.query.unwrap().location;
    assert!(query.contains("(compound_statement) @target") && query.contains("(#eq? @name \"MethodOne\")"), "{query}");
}

#[test]
fn queries_name_their_language() {
    assert_eq!(TreeSitterBackend::cpp().query_language().unwrap().name, "tree-sitter");
}

#[test]
fn names_follow_their_fields() {
    let found = suggested(CPP, "MethodTwo");

    assert_eq!(paths(&found)[..2], [("function_declarator.declarator", 2), ("function_definition.declarator.declarator", 2)]);
    assert!(found.query.unwrap().location.contains("(#eq? @target \"MethodTwo\")"));
}

#[test]
fn blocks_without_a_field_follow_their_parents() {
    let found = suggested(CPP, "int a = 1;");

    assert_eq!(paths(&found), vec![("declaration", 2), ("compound_statement.declaration", 2), ("function_definition.body.declaration", 2)]);
}

#[test]
fn unique_blocks_need_no_query() {
    let found = suggested(CPP, "<std>");

    assert_eq!(paths(&found)[0], ("preproc_include.path", 1));
    assert_eq!(found.query, None);
}

#[test]
fn equal_blocks_have_no_query() {
    let twice = format!("{CPP}{}", &CPP[CPP.find("void MethodOne").unwrap()..]);

    check(&TreeSitterBackend::cpp(), &twice);
    let found = suggested(&twice, "MethodOne");
    assert_eq!(paths(&found)[0], ("function_declarator.declarator", 4));
    assert_eq!(found.query, None);
}

#[test]
fn awkward_sources_are_suggested_for() {
    let backend = TreeSitterBackend::cpp();

    for source in [
        "", "\n", "void", "void a( {\n", "}}}", "class {{ void x() { \"é\" \u{1F600} }", "int a = \"é\"; int b = \"\\\"q\\\\\"; int c = \"\u{1F600}\";",
        "auto s = R\"(a\nb)\"; char c = '\\\\'; char d = '\"';", "void f() { x(\"tab\there\"); x(\"tab\tthere\"); }", "#define A(x) \\\n  x\nA(1)\nA(2)\n"
    ] {
        check(&backend, source);
        accepts_its_names(&backend, source);
    }

    assert_eq!(backend.suggest_locations("int a;", 0).unwrap(), Suggestions::default());
    assert_eq!(backend.suggest_locations("int a;", 1000).unwrap(), Suggestions::default());
}

#[test]
fn errors_are_marked() {
    let backend = TreeSitterBackend::cpp();
    let nodes = backend.nodes("void a() {}\nvoid b() { int = ; }\n").unwrap();
    let functions: Vec<bool> = nodes.iter().filter(|node| node.kind == "function_definition").map(|node| node.broken).collect();

    assert_eq!(functions, vec![false, true]);
    assert!(nodes[0].broken);
}
