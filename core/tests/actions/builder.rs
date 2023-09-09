// Standard Uses

// Crate Uses

// External Uses
/*
use codeshaper_general::ast;
use codeshaper_general::shaping::operation::actions;
use codeshaper_general::shaping::patch;
use codeshaper_general::shaping::patch::Actions;


#[allow(unused)]
#[test]
fn builder_build() {
    let input = "\
        Lets greet CodeShaper!
    ";

    let hello_builder = patch::builder::Builder {
        name: "hello_builder".to_string(),
        location: "All".to_string(),
        reference_location: None,
        r#match: "".to_string(),
        build: "Hello #[code_builder] to the World!".to_string(),

        actions: Actions {
            builders: Some(vec![
                patch::builder::Builder {
                    name: "code_builder".to_string(),
                    location: "All".to_string(),
                    reference_location: None,
                    r#match: "Lets greet (.*?)!".to_string(),
                    build: "\\g<1>".to_string(),
                    actions: Default::default(),
                }
            ]),
            replacers: None, makers: None, resolvers: None,
        },
    };

    let hello_builder_action = actions::builder::Builder::with(hello_builder);

    let patch_controller = patch::controller::Controller::from_patch(patch);

    let mut listener_controller = ast::listener::Controller::default();

    listener_controller.visit();


    assert_eq!(
        patch_controller.find_action("hello_builder").unwrap().name(),
        "Hello CodeShaper to the World!"
    )
}
*/
