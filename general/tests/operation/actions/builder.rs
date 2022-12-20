// Standard Uses
use std::rc::Rc;
use std::cell::RefCell;

// Crate Uses
use codeshaper_general::ast;
use codeshaper_general::shaping::patch;
use codeshaper_general::shaping::patch::{Actions, Patch};

// External Uses

#[allow(unused)]
#[test]
fn builder_expression() {

    let hello_builder = patch::builder::Builder {
        name: "hello_builder".to_string(),
        location: "function".to_string(),
        build: "Hello (.*)!".to_string(),
        reference_location: None,
        r#match: "".to_string(),
        actions: Default::default(),
    };

    let mut patch = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: Some(vec![hello_builder]),
            replacers: Default::default(),
            makers: Default::default(),
            resolvers: Default::default(),
        },
    };

    let mut listener_controller = Rc::new(RefCell::new(
        ast::listener::Controller::default()
    ));

    let patch_controller = patch::controller::Controller {
        listener_controller: listener_controller.clone(),
        patch: Rc::new(patch),
        actions: vec![],
    };

    listener_controller.borrow_mut().process();

    // assert_eq!(builder_expr.result(), &Some("Hello Rust!".to_string()));
}
