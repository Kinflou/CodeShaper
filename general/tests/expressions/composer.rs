// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses

// External Uses
use codeshaper_general::shaping::patch;
use codeshaper_general::shaping::operation::actions;
use codeshaper_general::shaping::patch::Actions;


#[allow(unused)]
#[test]
fn simple_composition_without_make_action_expression() {
    let input = "Hello Composition attempt";

    let mut hello_builder_action = Rc::new(RefCell::new(
        actions::builder::Builder::with_shared(patch::builder::Builder {
            name: "hello_builder".to_string(),
            location: "Any".to_string(),
            reference_location: None,
            r#match: "\"Hello (.*?) attempt\"".to_string(),
            build: "Goodbye $1 #[call](\"e\") World".to_string(),
            actions: Default::default(),
        })
    ));

    let mut bor = hello_builder_action.borrow_mut();
    let result = bor.process_input(input.to_string());

    println!("{:?}", bor.built);
    assert_eq!(bor.built, Some("Goodbye Expression World!".to_string()));
}


#[allow(unused)]
#[test]
fn simple_composition_with_build_action_expression() {
    let input = "Hello Action attempt";

    let mut hello_builder_action = Rc::new(RefCell::new(
        actions::builder::Builder::with_shared(patch::builder::Builder {
            name: "hello_builder".to_string(),
            location: "Any".to_string(),
            reference_location: None,
            r#match: "\"Hello (.*?) attempt\"".to_string(),
            build: "Goodbye #[greet_maker]($1) World".to_string(),
            actions: Actions {
                builders: None,
                replacers: None,
                makers: Some(vec![
                    patch::maker::Maker {
                        name: "greet_maker".to_string(),
                        prepare: None,
                        make: "Maker $[1]".to_string(),
                    }
                ]),
                resolvers: None,
            }
        })
    ));

    let mut bor = RefCell::borrow_mut(&hello_builder_action);
    let result = bor.process_input(input.to_string());

    println!("{:?}", bor.built);
    assert_eq!(bor.built, Some("Goodbye Maker Action World".to_string()));
}

