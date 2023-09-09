// Standard Uses
use std::cell::RefCell;
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_general::shaping::patch;
use codeshaper_general::shaping::operation::actions;
use codeshaper_general::shaping::operation::actions::analyzer;
use codeshaper_general::shaping::operation::actions::analyzer::{Analyzer, Error};


#[allow(unused)]
#[test]
fn analyze_action_location_not_found() {
    let hello_builder_action = Rc::new(
        RefCell::new(actions::builder::Builder::with_shared(
            patch::builder::Builder {
                name: "hello_builder".to_string(),
                location: "non-existent".to_string(),
                reference_location: None,
                r#match: "Hello (.*?) World!".to_string(),
                build: "Bye $[1] ...".to_string(),
                actions: Default::default(),
            }
        ))
    );

    let mut analyzer = Rc::new(RefCell::new(
        analyzer::Analyzer::with_action_shared_cell(&hello_builder_action)
    ));

    analyzer.analyze();
    // Analyzer::analyze(&mut analyzer);

    println!("Errors: {:?}", analyzer.borrow().errors);
    assert_eq!(vec![Error::LocationNotFound], analyzer.borrow().errors);
}


#[allow(unused)]
#[test]
fn analyze_action_action_not_found() {
    let hello_builder_action = Rc::new(RefCell::new(actions::builder::Builder::with_shared(
        patch::builder::Builder {
            name: "hello_builder".to_string(),
            location: "*".to_string(),
            reference_location: None,
            r#match: "Hello (.*?) World!".to_string(),
            build: "Bye #[undefined_action] ...".to_string(),
            actions: Default::default(),
        }
    )));

    let analyzer = Rc::new(RefCell::new(
        analyzer::Analyzer::with_action_shared_mut(hello_builder_action)
    ));

    Analyzer::analyze(&analyzer);

    println!("Errors: {:?}", analyzer.borrow().errors);
    assert_eq!(vec![Error::ActionNotFound], analyzer.borrow().errors);
}

