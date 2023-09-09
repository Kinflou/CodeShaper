// Standard Uses

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch;
use codeshaper_core::shaping::patch::controller::PatchController;


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

    let mut patch = patch::Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: patch::Actions {
            builders: Some(vec![hello_builder]),
            replacers: Default::default(),
            makers: Default::default(),
            resolvers: Default::default(),
        },
    };

    let mut patch_controller = PatchController::with(patch);
    patch_controller.listener_controller.process();

    // assert_eq!(builder_expr.result(), &Some("Hello Rust!".to_string()));
}
