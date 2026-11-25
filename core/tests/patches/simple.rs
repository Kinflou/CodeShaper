// Standard Uses
use std::path::Path;
use std::sync::LazyLock;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch::{Patch, Actions};
use codeshaper_core::shaping::patch::builder::Builder;
use codeshaper_core::shaping::patch::maker::Maker;


#[allow(unused)]
#[test]
fn load_patch_kdl() {
    let path = Path::new("tests/data/patches/patch.kdl");

    let patch = Patch::from_path(path).unwrap();

    pretty_assertions::assert_eq!(*EXPECTED_PATCH, patch);
}

#[test]
fn invalid_patch_reports_labelled_error() {
    let content = "enabled \"yes\"\nalias \"text\"\nfile \"test.txt\"\nactions {\n}\n";

    let report = Patch::from_kdl("broken.kdl", content).unwrap_err();

    assert!(report.source_code().is_some());
    let mut related = report.related().unwrap();
    assert!(related.any(|error| error.labels().is_some_and(|mut labels| labels.next().is_some())));
}

pub static EXPECTED_PATCH: LazyLock<Patch> = LazyLock::new(|| {
    Patch {
        enabled: true,
        alias: "text".to_owned(),
        file: "test.txt".to_owned(),
        actions: Actions {
            builders: vec![
                Builder {
                    name: "hello_builder".to_owned(),
                    location: "*".to_string(),
                    reference_location: None,
                    r#match: "Lets greet (.+)!".to_owned(),
                    flags: String::new(),
                    build: "Hello #[code_maker($[0])] World!".to_owned(),
                    collect: false,
                    separator: String::new(),
                    actions: Default::default(),
                }
            ],
            replacers: Default::default(),
            makers: vec![
                Maker {
                    name: "code_maker".to_owned(),
                    prepare: None,
                    flags: String::new(),
                    make: "@[0] Library".to_owned(),
                }
            ],
            switches: Default::default(),
            lists: Default::default(),
        },
    }
});
