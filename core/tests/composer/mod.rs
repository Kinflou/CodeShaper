// Relative Modules
mod builders;
mod indexers;
mod makers;

// Standard Uses
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch;
use codeshaper_core::shaping::patch::{Actions, Patch};
use codeshaper_core::shaping::patch::builder::Builder;
use codeshaper_core::shaping::patch::controller::PatchController;


#[allow(unused)]
#[test]
fn composition_builder_with_undefined_action() {
    let input = "Hello Composition attempt";

    let mut hello_builder = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: Some(vec![
                Builder {
                    name: "hello_builder".to_string(),
                    location: "Any".to_string(),
                    reference_location: None,
                    r#match: "Hello (.*) attempt".to_string(),
                    build: "Goodbye $[0] #[undefined_action] World".to_string(),
                    actions: Default::default(),
                }
            ]),
            replacers: None,
            makers: None,
            switchers: None,
            lists: None
        },
    };
    let hello_builder = Rc::new(hello_builder);

    let mut patch_controller = PatchController::with(hello_builder);

    let mut hello_builder_ref = patch_controller
        .find_action("hello_builder", &patch_controller)
        .unwrap().builder().unwrap();

    let result = hello_builder_ref.process_input(
        &patch_controller, input.to_owned()
    );

    assert_eq!(None, result);
}


#[allow(unused)]
#[test]
fn simple_composition_with_build_action_expression() {
    let input = "Hello Action attempt";

    let mut hello_builder = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: Some(vec![
                Builder {
                    name: "hello_builder".to_owned(),
                    location: "Any".to_owned(),
                    reference_location: None,
                    r#match: "Hello (.+) attempt".to_owned(),
                    build: "Goodbye #[greet_maker]($[0]) World".to_owned(),
                    actions: Actions {
                        builders: None,
                        replacers: None,
                        makers: Some(vec![
                            patch::maker::Maker {
                                name: "greet_maker".to_owned(),
                                prepare: None,
                                make: "Maker @[0]".to_owned(),
                            }
                        ]),
                        switchers: None, lists: None
                    }
                }
            ]),
            replacers: None,
            makers: None,
            switchers: None, lists: None
        },
    };
    let hello_builder = Rc::new(hello_builder);

    let mut patch_controller = PatchController::with(hello_builder);

    let mut hello_builder_ref = patch_controller
        .find_action("hello_builder", &patch_controller)
        .unwrap().builder().unwrap();

    let result = hello_builder_ref.process_input(
        &patch_controller, input.to_owned()
    );

    assert_eq!(Some("Goodbye Maker Action World".to_owned()), result);
}

