// Standard USes
use std::rc::Rc;
use std::collections::HashMap;

// Crate Uses

// External Uses
use codeshaper_core::ast::listener::ListenerController;
use codeshaper_core::shaping::patch::{Actions, Patch};
use codeshaper_core::shaping::patch::builder::Builder;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::operation::actions::Action;


#[test]
fn composition_with_single_builder_in_root_builder() {
    let hello_builder = Patch {
        enabled: false,
        alias: "".to_owned(),
        file: "".to_owned(),
        actions: Actions {
            builders: Some(vec![
                Builder {
                    name: "pool_builder".to_owned(),
                    location: "Area".to_owned(),
                    reference_location: None,
                    r#match: "empty".to_owned(),
                    flags: String::new(),
                    build: "A pool of #[pool_mud_builder]".to_owned(),
                    collect: false,
                    separator: String::new(),
                    actions: Actions {
                        builders: Some(vec![
                            Builder {
                                name: "pool_mud_builder".to_owned(),
                                location: "Pool".to_owned(),
                                reference_location: None,
                                r#match: "made of (.+)".to_owned(),
                                flags: String::new(),
                                build: "$[0]".to_owned(),
                                collect: false,
                                separator: String::new(),
                                actions: Default::default(),
                            }
                        ]),
                        replacers: None,
                        makers: None,
                        switchers: None,
                        lists: None,
                    },
                }
            ]),
            replacers: None,
            makers: None,
            switchers: None,
            lists: None
        },
    };
    let hello_builder = Rc::new(hello_builder);

    let mut listener_controller = ListenerController::default();
    listener_controller.location_content = HashMap::from([
        ("Area".to_owned(), "empty".to_owned()),
        ("Pool".to_owned(), "made of mud".to_owned())
    ]);

    let patch_controller = PatchController::with_listener(
        hello_builder, listener_controller
    );

    let hello_builder_ref = patch_controller
        .find_action("pool_builder", &patch_controller)
        .unwrap().builder().unwrap();


    let result = hello_builder_ref.process(&patch_controller);

    assert_eq!(Some("A pool of mud".to_string()), result);
}

#[test]
fn composition_with_single_builder_in_root_builder_missing_location() {
    let hello_builder = Patch {
        enabled: false,
        alias: "".to_owned(),
        file: "".to_owned(),
        actions: Actions {
            builders: Some(vec![
                Builder {
                    name: "pool_builder".to_owned(),
                    location: "Area".to_owned(),
                    reference_location: None,
                    r#match: "empty".to_owned(),
                    flags: String::new(),
                    build: "A pool of #[pool_mud_builder]".to_owned(),
                    collect: false,
                    separator: String::new(),
                    actions: Actions {
                        builders: Some(vec![
                            Builder {
                                name: "pool_mud_builder".to_owned(),
                                location: "Pool".to_owned(),
                                reference_location: None,
                                r#match: "made of (.+)".to_owned(),
                                flags: String::new(),
                                build: "$[0]".to_owned(),
                                collect: false,
                                separator: String::new(),
                                actions: Default::default(),
                            }
                        ]),
                        replacers: None,
                        makers: None,
                        switchers: None,
                        lists: None,
                    },
                }
            ]),
            replacers: None,
            makers: None,
            switchers: None,
            lists: None
        },
    };
    let hello_builder = Rc::new(hello_builder);

    let mut listener_controller = ListenerController::default();
    listener_controller.location_content = HashMap::from([
        ("Area".to_owned(), "empty".to_owned()),
    ]);

    let patch_controller = PatchController::with_listener(
        hello_builder, listener_controller
    );

    let hello_builder_ref = patch_controller
        .find_action("pool_builder", &patch_controller)
        .unwrap().builder().unwrap();


    let result = hello_builder_ref.process(&patch_controller);

    assert_eq!(None, result);
    assert_eq!(Some("pool_builder"), hello_builder_ref.current_inner_builder.borrow().as_deref());
}

