// Standard Uses

// Crate Uses

// External Uses
#[allow(unused_imports)]
use codeshaper::registry;


#[cfg(all(feature = "cpp", feature = "text"))]
#[test]
fn default_plugins_registered() {
    let registry = registry();

    assert_eq!(registry.backend("cpp").unwrap().name(), "cpp");
    assert_eq!(registry.loader("text").unwrap().name(), "text");
}

#[cfg(all(feature = "gdscript", feature = "godot"))]
#[test]
fn godot_plugins_registered() {
    let registry = registry();

    assert_eq!(registry.backend("gdscript").unwrap().name(), "gdscript");
    assert_eq!(registry.backend("godot_resource").unwrap().name(), "godot_resource");
    assert_eq!(registry.loader("godot").unwrap().name(), "godot");
}
