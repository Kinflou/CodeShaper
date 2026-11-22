// Standard Uses
use std::fs;
use std::path::Path;
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::registry::Registry;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_nav_ast_treesitter::{register, TreeSitterBackend};


const EXAMPLE: &str = "../../codeshaper-rs/examples/CPP14/Function Name Correction";

fn example_file(path: &str) -> String {
    fs::read_to_string(Path::new(EXAMPLE).join(path)).unwrap().replace("\r\n", "\n")
}

fn example_source(name: &str) -> String {
    example_file(&format!("target/{name}"))
}

fn apply_example_patch(patch: &str, source: &str) -> String {
    let path = Path::new(EXAMPLE).join("shaping_project/patches").join(patch);
    let patch = Patch::from_path(&path).unwrap();

    PatchController::with(Rc::new(patch))
        .apply(&TreeSitterBackend::cpp(), source)
        .unwrap()
}

fn located<'a>(source: &'a str, location: &str) -> Vec<&'a str> {
    TreeSitterBackend::cpp()
        .locate(source, location)
        .unwrap()
        .into_iter()
        .map(|range| &source[range])
        .collect()
}


#[test]
fn locate_function_definitions() {
    let source = example_source("Module.cpp");
    let found = located(&source, "FunctionDefinition");

    assert_eq!(found.len(), 2);
    assert!(found[0].starts_with("void __HelloWorld()"));
    assert!(found[1].starts_with("void __HelloPerson()"));
}

#[test]
fn locate_function_bodies() {
    let source = example_source("Module.cpp");
    let found = located(&source, "FunctionDefinition.body");

    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|body| body.starts_with('{') && body.ends_with('}')));
}

#[test]
fn locate_query() {
    let source = example_source("Module.cpp");
    let found = located(
        &source,
        "(function_definition declarator: (function_declarator declarator: (identifier) @target))"
    );

    assert_eq!(found, vec!["__HelloWorld", "__HelloPerson"]);
}

#[test]
fn locate_unknown_kind() {
    let source = example_source("Module.cpp");

    assert!(TreeSitterBackend::cpp().locate(&source, "FunctionBody").is_err());
}

#[test]
fn locate_query_without_target() {
    let source = example_source("Module.cpp");

    assert!(TreeSitterBackend::cpp().locate(&source, "(function_definition)").is_err());
}

#[test]
fn function_name_correction() {
    let source = example_source("Module.cpp");
    let result = apply_example_patch("function_name_correction_cpp.kdl", &source);

    assert_eq!(result, source.replace("void __", "void _"));
}

#[test]
fn function_return_creation() {
    let source = example_source("ReturnModule.cpp");
    let result = apply_example_patch("function_return_creation.kdl", &source);

    assert_eq!(result, example_file("expected_result/ReturnModule.cpp"));
}

#[test]
fn function_creation() {
    let source = example_source("CallerModule.cpp");
    let result = apply_example_patch("function_creation.kdl", &source);

    assert_eq!(result, example_file("expected_result/CallerModule.cpp"));
}

#[test]
fn example_patches_load() {
    let patches = Path::new(EXAMPLE).join("shaping_project/patches");

    for entry in fs::read_dir(patches).unwrap() {
        let path = entry.unwrap().path();

        assert!(Patch::from_path(&path).is_ok(), "{} does not load", path.display());
    }
}

#[test]
fn registers_cpp_extensions() {
    let mut registry = Registry::default();
    register(&mut registry);

    assert_eq!(registry.backend("cpp").unwrap().name(), "cpp");
    assert_eq!(registry.backend_for(Path::new("Module.cpp")).unwrap().name(), "cpp");
    assert_eq!(registry.backend_for(Path::new("core/object/object.inc")).unwrap().name(), "cpp");
    assert!(registry.backend_for(Path::new("main.rs")).is_none());
}

#[test]
fn check_locations() {
    let backend = TreeSitterBackend::cpp();

    assert!(backend.check_location("FunctionDefinition.body").is_ok());
    assert!(backend.check_location("(function_definition declarator: (_) @target)").is_ok());
    assert!(backend.check_location("FunctionBody").is_err());
    assert!(backend.check_location("Body").is_err());
    assert!(backend.check_location("(function_definition)").is_err());
    assert!(backend.check_location("(function_definition").is_err());
}
