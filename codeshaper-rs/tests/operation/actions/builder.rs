// Standard Uses

// Crate Uses
use codeshaper::ast::listener::Controller;
use codeshaper::shaping::operation::actions::Action;
use codeshaper::shaping::operation::actions::builder::Builder as BuilderExpr;
use codeshaper::shaping::patch::controller::PatchController;
use codeshaper::shaping::patch::Builder as PatchBuilder;

// External Uses


#[test]
fn builder_expression() {
    let controller = Controller::default();
    let patch_controller = PatchController {};

    let patch_builder = PatchBuilder {
        location: "function".to_string(),
        build: "Hello (.*)!".to_string(),
        reference_location: String::default(),
        r#match: "".to_string(),
        actions: Default::default(),
    };
    
    let mut builder_expr = BuilderExpr {
        name: "hello_builder".to_string(),
        builder: patch_builder,
        controller: patch_controller,
        built: false,
        location: None,
        result: String::default(),
    };

    builder_expr.process(&controller);

    assert_eq!(builder_expr.built, true);
    assert_eq!(builder_expr.result, "Hello Rust!");
}