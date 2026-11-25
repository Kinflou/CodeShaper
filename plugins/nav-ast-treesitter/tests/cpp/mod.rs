// Relative Modules
mod referred;

// Standard Uses
use std::fs;
use std::path::Path;
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::{LanguageBackend, LocationError};
use codeshaper_core::registry::Registry;
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::validation::{validate, Severity};
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
    let found = located(&source, "function_definition");

    assert_eq!(found.len(), 2);
    assert!(found[0].starts_with("void __HelloWorld()"));
    assert!(found[1].starts_with("void __HelloPerson()"));
}

#[test]
fn locate_function_bodies() {
    let source = example_source("Module.cpp");
    let found = located(&source, "function_definition.body");

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
fn registers_cpp() {
    let mut registry = Registry::default();
    register(&mut registry);

    assert_eq!(registry.backend("cpp").unwrap().name(), "cpp");
}

#[test]
fn check_locations() {
    let backend = TreeSitterBackend::cpp();

    assert!(backend.check_location("function_definition.body").is_ok());
    assert!(backend.check_location("(function_definition declarator: (_) @target)").is_ok());
    assert!(backend.check_location("FunctionBody").is_err());
    assert!(backend.check_location("body").is_err());
    assert!(backend.check_location("(function_definition)").is_err());
    assert!(backend.check_location("(function_definition").is_err());
}

#[test]
fn example_patches_validate() {
    let patches = Path::new(EXAMPLE).join("shaping_project/patches");
    let backend = TreeSitterBackend::cpp();

    for entry in fs::read_dir(patches).unwrap() {
        let path = entry.unwrap().path();
        let patch = Patch::from_path(&path).unwrap();

        let errors: Vec<_> = validate(&patch, [&backend as &dyn LanguageBackend], &[]).into_iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .collect();

        assert!(errors.is_empty(), "{}: {errors:#?}", path.display());
    }
}

fn location_range(location: &str) -> Option<String> {
    let error = TreeSitterBackend::cpp().check_location(location).unwrap_err();

    error.downcast_ref::<LocationError>().unwrap().range.clone().map(|range| location[range].to_string())
}

#[test]
fn location_errors_point_at_the_wrong_part() {
    assert_eq!(location_range("function_definition.nope.body").as_deref(), Some("nope"));
    assert_eq!(location_range("  Nope.body").as_deref(), Some("Nope"));
    assert_eq!(location_range("\n  (function_definition\n    (nope) @target)").as_deref(), Some("nope"));
    assert_eq!(location_range("(function_definition nope: (_) @target)").as_deref(), Some("nope"));
    assert_eq!(location_range("((identifier) @target (#eq? @other \"a\"))").as_deref(), Some("other"));
}

#[test]
fn only_the_grammars_names_are_known() {
    let backend = TreeSitterBackend::cpp();

    for (location, wrong) in [
        ("FunctionDefinition", "FunctionDefinition"), ("functionDefinition", "functionDefinition"), ("Function_Definition", "Function_Definition"),
        ("function_definition.Body", "Body"), ("class_specifier.body.FieldDeclaration", "FieldDeclaration")
    ] {
        assert_eq!(location_range(location).as_deref(), Some(wrong));
        assert_eq!(backend.check_location(location).unwrap_err().to_string(), format!("'{wrong}' is not a cpp node kind or field"));
    }
    assert!(backend.check_location(" function_definition . body ").is_ok());
}

#[test]
fn paths_and_queries_find_the_same() {
    let backend = TreeSitterBackend::cpp();
    let sources = ["CallerModule.cpp", "Module.cpp", "Module.h", "ReturnModule.cpp"].map(example_source);
    let broken = "class A { void a() { if (b) { c(); } } };\nint x = ;\nvoid d( {\n".to_string();
    let mut found = 0;

    for source in sources.iter().chain([&broken]) {
        for kind in backend.node_kinds().iter().map(String::as_str).chain(["ERROR"]) {
            let places = backend.locate(source, kind).unwrap();
            let queried = backend.locate(source, &format!("({kind}) @target")).unwrap();

            assert_eq!(places, queried, "{kind}");
            found += places.len();
        }
    }

    assert!(found > 100);
    assert!(!backend.locate(&broken, "ERROR").unwrap().is_empty());
}

#[test]
fn locate_the_outermost_block() {
    let source = "int a;\nvoid f() { int b; }\n";

    assert_eq!(located(source, "translation_unit"), [source]);
    assert_eq!(located(source, "translation_unit.declaration"), ["int a;", "int b;"]);
    assert_eq!(located(source, "translation_unit.function_definition.body"), located(source, "function_definition.body"));
    assert_eq!(located(source, "declaration.translation_unit"), [""; 0]);
    assert_eq!(located(source, "translation_unit.translation_unit"), [""; 0]);
    assert_eq!(located("", "translation_unit"), [""]);
}

#[test]
fn locate_a_file_that_is_one_error() {
    let source = "} ) ]";
    let errors = located(source, "ERROR");

    assert_eq!(errors, located(source, "(ERROR) @target"));
    assert!(errors.contains(&source));
}

#[test]
fn locations_found_together_are_the_same() {
    let backend = TreeSitterBackend::cpp();
    let source = "class A { void a() { if (b) { c(); } } };\nvoid d() {}\nint x = ;\n";
    let kinds = backend.node_kinds();
    let mut locations: Vec<&str> = kinds.iter().map(String::as_str).collect();
    locations.extend([
        "function_definition.body", "function_definition.function_declarator.declarator", "Nothing", "", "(", "(identifier) @other",
        "(identifier) @target", "(ERROR) @target", "(call_expression function: (identifier) @target)", "(identifier) @target"
    ]);

    let text = |found: Result<Vec<std::ops::Range<usize>>, miette::Report>| found.map_err(|error| error.to_string());
    let alone: Vec<_> = locations.iter().map(|location| text(backend.locate(source, location))).collect();
    let together: Vec<_> = backend.locate_all(source, &locations).into_iter().map(text).collect();

    assert_eq!(together, alone);
    assert!(alone.iter().any(|found| found.is_err()) && alone.iter().any(|found| found.as_ref().is_ok_and(|found| found.len() > 2)));
    assert!(backend.locate_all(source, &[]).is_empty());
}

#[test]
fn many_queries_stay_right() {
    let backend = TreeSitterBackend::cpp();
    let source = (0..150).map(|number| format!("void f{number}() {{}}\n")).collect::<String>();

    for number in (0..150).chain([0, 149, 3]) {
        let query = format!("((identifier) @target (#eq? @target \"f{number}\"))");
        let found = backend.locate(&source, &query).unwrap();

        assert_eq!(found.into_iter().map(|range| &source[range]).collect::<Vec<_>>(), vec![format!("f{number}")]);
        assert!(backend.locate(&source, "((identifier) @other)").is_err());
    }
}
