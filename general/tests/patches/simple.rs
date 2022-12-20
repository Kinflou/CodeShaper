// Standard Uses
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_general::shaping::patch::{Patch, Actions};
use codeshaper_general::shaping::patch::builder::Builder;
use codeshaper_general::shaping::patch::maker::Maker;
use lazy_static::lazy_static;


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

    assert_eq!(*EXPECTED_PATCH, patch);
}

lazy_static!(
    pub static ref EXPECTED_PATCH: Patch = {
        Patch {
            enabled: true,
            alias: "text".to_string(),
            file: "test.txt".to_string(),
            actions: Actions {
                builders: Some(vec![
                    Builder {
                        name: "hello_builder".to_string(),
                        location: "*".to_string(),
                        reference_location: None,
                        r#match: "Lets greet (.*?)!".to_string(),
                        build: "Hello #[code_maker](\\g<1>) World!".to_string(),
                        actions: Default::default(),
                    }
                ]),
                replacers: Default::default(),
                makers: Some(vec![
                    Maker {
                        name: "code_maker".to_string(),
                        prepare: None,
                        make: "#[0] Library".to_string(),
                    }
                ]),
                resolvers: Default::default(),
            },
        }
    };
);
