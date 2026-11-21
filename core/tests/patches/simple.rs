// Standard Uses
use std::path::Path;
use std::sync::LazyLock;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch::{Patch, Actions};
use codeshaper_core::shaping::patch::builder::Builder;
use codeshaper_core::shaping::patch::maker::Maker;


#[allow(unused)]
// #[test]
fn load_patch_json5() {
    let path = Path::new("tests/data/patches/patch.json5");

    let patch = Patch::from_path(path).unwrap();

    assert_eq!(*EXPECTED_PATCH, patch);
}


#[allow(unused)]
#[test]
fn load_patch_kdl() {
    let path = Path::new("tests/data/patches/patch.kdl");

    let patch = Patch::from_path(path).unwrap();

    pretty_assertions::assert_eq!(*EXPECTED_PATCH, patch);
}

pub static EXPECTED_PATCH: LazyLock<Patch> = LazyLock::new(|| {
    Patch {
        enabled: true,
        alias: "text".to_owned(),
        file: "test.txt".to_owned(),
        actions: Actions {
            builders: Some(vec![
                Builder {
                    name: "hello_builder".to_owned(),
                    location: "*".to_string(),
                    reference_location: None,
                    r#match: "Lets greet (.+)!".to_owned(),
                    build: "Hello #[code_maker]($[0]) World!".to_owned(),
                    collect: false,
                    separator: String::new(),
                    actions: Default::default(),
                }
            ]),
            replacers: Default::default(),
            makers: Some(vec![
                Maker {
                    name: "code_maker".to_owned(),
                    prepare: None,
                    make: "@[0] Library".to_owned(),
                }
            ]),
            switchers: Default::default(),
            lists: Default::default(),
        },
    }
});
