// Standard Uses
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::File;


const EXAMPLE_TARGET: &str = "../../codeshaper-rs/examples/CPP14/Function Name Correction/target/";

fn registry() -> Registry {
    let mut registry = Registry::default();
    codeshaper_target_text::register(&mut registry);

    registry
}

fn file(path: &str) -> File {
    File { path: path.into(), language: None }
}


#[test]
fn loads_example() {
    let solution = registry().load_target("text", Path::new(EXAMPLE_TARGET)).unwrap();

    assert_eq!(solution.name, "Function Name Correction");
    assert_eq!(solution.directory, Path::new(EXAMPLE_TARGET));
    assert_eq!(solution.groups.len(), 1);
    assert_eq!(solution.groups[0].name, "root");
    assert_eq!(
        solution.files(),
        vec![&file("CallerModule.cpp"), &file("ReturnModule.cpp"), &file("Module.cpp"), &file("Module.h")]
    );
}

#[test]
fn missing_file() {
    let error = registry().load_target("text", Path::new("tests/data/missing_file/")).unwrap_err();

    assert!(format!("{error:?}").contains("'Missing.cpp'"));
}

#[test]
fn missing_project() {
    assert!(registry().load_target("text", Path::new("tests/data/")).is_err());
}
