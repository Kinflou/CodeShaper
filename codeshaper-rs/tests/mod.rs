#![cfg(all(feature = "cpp", feature = "text"))]

// Standard Uses
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper::registry;


const EXAMPLE_TARGET: &str = "examples/CPP14/Function Name Correction/target/";


#[test]
fn default_plugins_registered() {
    let registry = registry();

    assert_eq!(registry.backend("cpp").unwrap().name(), "cpp");
    assert_eq!(registry.loader("text").unwrap().name(), "text");
}

#[test]
fn example_target_files_have_backends() {
    let registry = registry();
    let solution = registry.load_target("text", Path::new(EXAMPLE_TARGET)).unwrap();

    assert_eq!(solution.files().len(), 4);

    for file in solution.files() {
        let backend = registry.backend_for(&file.path).unwrap();

        assert_eq!(backend.name(), "cpp", "{} has no cpp backend", file.path.display());
    }
}
