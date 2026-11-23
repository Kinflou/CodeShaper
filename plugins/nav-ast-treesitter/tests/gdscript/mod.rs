// Standard Uses

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::registry::Registry;
use codeshaper_nav_ast_treesitter::{register, TreeSitterBackend};


const PLAYER: &str = "\
extends CharacterBody2D

@export var speed: float = 200.0
var health := 3

func _ready() -> void:
\tprint(\"ready\")

func take_damage(amount: int) -> void:
\thealth -= amount
";

fn located<'a>(source: &'a str, location: &str) -> Vec<&'a str> {
    TreeSitterBackend::gdscript()
        .locate(source, location)
        .unwrap()
        .into_iter()
        .map(|range| &source[range])
        .collect()
}


#[test]
fn locate_function_definitions() {
    let found = located(PLAYER, "FunctionDefinition");

    assert_eq!(found.len(), 2);
    assert!(found[0].starts_with("func _ready()"));
    assert!(found[1].starts_with("func take_damage(amount: int)"));
}

#[test]
fn locate_function_names() {
    assert_eq!(located(PLAYER, "FunctionDefinition.name"), vec!["_ready", "take_damage"]);
}

#[test]
fn locate_exported_variables() {
    let found = located(
        PLAYER,
        r#"(variable_statement (annotations (annotation (identifier) @kind)) name: (name) @target (#eq? @kind "export"))"#
    );

    assert_eq!(found, vec!["speed"]);
}

#[test]
fn registers_gdscript() {
    let mut registry = Registry::default();
    register(&mut registry);

    assert_eq!(registry.backend("gdscript").unwrap().name(), "gdscript");
}

#[test]
fn check_locations() {
    let backend = TreeSitterBackend::gdscript();

    assert!(backend.check_location("VariableStatement.name").is_ok());
    assert!(backend.check_location("ExportVariableStatement").is_ok());
    assert!(backend.check_location("ExportStatement").is_err());
}
