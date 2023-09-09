// Standard Uses
use std::cell::RefCell;
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_general::shaping::operation::actions;
use codeshaper_general::shaping::operation::actions::linter;
use codeshaper_general::shaping::operation::actions::linter::{Linter, Warning};
use codeshaper_general::shaping::patch;
use codeshaper_general::shaping::patch::Actions;


#[allow(unused)]
#[test]
fn lint_action() {
    let hello_builder_action = Box::new(actions::builder::Builder::with_shared(
        patch::builder::Builder {
            name: "hello_builder".to_string(),
            location: "hello_builder_action".to_string(),
            reference_location: None,
            r#match: "Hello (.*?) World!".to_string(),
            build: "\"Bye \" $1 \"...\"".to_string(),
            actions: Actions {
                builders: Some(vec![
                    patch::builder::Builder {
                        name: "unused_action".to_string(),
                        location: "*".to_string(),
                        reference_location: None,
                        r#match: "".to_string(),
                        build: "".to_string(),
                        actions: Default::default(),
                    }
                ]),
                replacers: None,
                makers: None,
                resolvers: None,
            },
        }
    ));

    let mut linter = Rc::new(RefCell::new(
        linter::Linter::with_action(hello_builder_action)
    ));

    Linter::lint(&mut linter);

    println!("Warnings {:?}", linter.borrow().warnings);
    assert_eq!(vec![Warning::UnusedAction], linter.borrow().warnings);
}
