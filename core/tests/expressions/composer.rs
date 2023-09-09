// Standard Uses

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch;
use codeshaper_core::shaping::operation::actions;
use codeshaper_core::shaping::patch::Actions;


#[allow(unused)]
#[test]
fn simple_composition_without_make_action_expression() {
    let input = "Hello Composition attempt";

    let mut hello_builder =
        actions::builder::BuilderAction::with(patch::builder::Builder {
            name: "hello_builder".to_string(),
            location: "Any".to_string(),
            reference_location: None,
            r#match: "Hello (.*) attempt".to_string(),
            build: "Goodbye $[1] #[undefined_action] World".to_string(),
            actions: Default::default(),
        }
    );
    let mut hello_builder_mut = hello_builder;
    let result = hello_builder_mut.process_input(input.to_string()).unwrap();

    println!("{:?}", result);
    assert_eq!(result, Some("Goodbye Expression World!".to_string()));
}


#[allow(unused)]
#[test]
fn simple_composition_with_build_action_expression() {
    let input = "Hello Action attempt";

    let mut hello_builder =
        actions::builder::BuilderAction::with(patch::builder::Builder {
            name: "hello_builder".to_owned(),
            location: "Any".to_owned(),
            reference_location: None,
            r#match: "Hello (.*) attempt".to_owned(),
            build: "Goodbye #[greet_maker]($[1]) World".to_owned(),
            actions: Actions {
                builders: None,
                replacers: None,
                makers: Some(vec![
                    patch::maker::Maker {
                        name: "greet_maker".to_owned(),
                        prepare: None,
                        make: "Maker @[1]".to_owned(),
                    }
                ]),
                resolvers: None,
            }
        }
    );
    // let patch_controller = PatchController::with(hello_builder);
    // let mut hello_builder_mut = hello_builder.borrow_mut();
    let mut hello_builder_mut = hello_builder;
    let result = hello_builder_mut.process_input(input.to_owned()).unwrap();

    println!("{:?}", result);
    assert_eq!(result, Some("Goodbye Maker Action World".to_owned()));
}

