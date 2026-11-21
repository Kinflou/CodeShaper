// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::shaping::patch::replacer::Replacer;
use codeshaper_nav_ast_treesitter::TreeSitterBackend;


const EXAMPLE_TARGET: &str = "../codeshaper-rs/examples/CPP14/Function Name Correction/target";

fn example_source(name: &str) -> String {
    fs::read_to_string(Path::new(EXAMPLE_TARGET).join(name)).unwrap()
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
    let replacer = Replacer {
        location: "FunctionDefinition".to_string(),
        from: r"void\s+?__(.*?)\(".to_string(),
        to: r"void _\g<1>(".to_string(),
        ..Default::default()
    };

    let result = replacer.apply(&TreeSitterBackend::cpp(), &source).unwrap();

    assert_eq!(result, source.replace("void __", "void _"));
}
