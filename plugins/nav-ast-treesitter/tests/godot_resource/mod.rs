// Standard Uses

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::registry::Registry;
use codeshaper_nav_ast_treesitter::{register, TreeSitterBackend};


const SCENE: &str = "\
[gd_scene load_steps=2 format=3]

[ext_resource type=\"Script\" path=\"res://player.gd\" id=\"1\"]

[node name=\"Player\" type=\"CharacterBody2D\"]
script = ExtResource(\"1\")
";

fn located<'a>(source: &'a str, location: &str) -> Vec<&'a str> {
    TreeSitterBackend::godot_resource()
        .locate(source, location)
        .unwrap()
        .into_iter()
        .map(|range| &source[range])
        .collect()
}


#[test]
fn locate_sections() {
    let found = located(SCENE, "Section");

    assert_eq!(found.len(), 3);
    assert!(found[2].starts_with("[node name=\"Player\""));
}

#[test]
fn locate_query() {
    let found = located(
        SCENE,
        r#"(section (identifier) @kind (attribute (identifier) @key (_) @target) (#eq? @kind "node") (#eq? @key "type"))"#
    );

    assert_eq!(found, vec!["\"CharacterBody2D\""]);
}

#[test]
fn registers_godot_resource() {
    let mut registry = Registry::default();
    register(&mut registry);

    assert_eq!(registry.backend("godot_resource").unwrap().name(), "godot_resource");
}
