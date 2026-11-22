// Standard Uses
use std::rc::Rc;
use std::collections::HashMap;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch::{Actions, Patch};
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::resolver::{List, Switch};


#[test]
fn composition_list_index_with_args() {
    let liquid_patch = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: None,
            replacers: None,
            makers: None,
            switchers: None,
            lists: Some(vec![
                List {
                    name: "liquid_list".to_string(),
                    values: vec![
                        "water".to_owned(), "oil".to_owned(), "soda".to_owned()
                    ],
                    index: Some("@[0]".to_owned()),
                    default: None,
                }
            ])
        },
    };
    let liquid_patch = Rc::new(liquid_patch);

    let patch_controller = PatchController::with(liquid_patch);

    let liquid_list = patch_controller
        .find_action("liquid_list", &patch_controller)
        .unwrap().list().unwrap();

    let result = liquid_list.index_with_args(
        &patch_controller, &["1".to_owned()]
    );

    assert_eq!(Some("oil".to_string()), result);
}


#[test]
fn composition_switch_with_args() {
    let number_patch = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: None,
            replacers: None,
            makers: None,
            switchers: Some(vec![
                Switch {
                    name: "number_switch".to_string(),
                    cases: HashMap::from([
                        ("1".to_owned(), "one".to_owned()),
                        ("2".to_owned(), "two".to_owned()),
                        ("3".to_owned(), "three".to_owned()),
                    ]),
                    index: "@[0]".to_owned(),
                    default: None,
                }
            ]),
            lists: None
        },
    };
    let number_patch = Rc::new(number_patch);

    let patch_controller = PatchController::with(number_patch);

    let number_switch = patch_controller
        .find_action("number_switch", &patch_controller)
        .unwrap().switch().unwrap();

    let result = number_switch.resolve(&patch_controller, "2");

    assert_eq!(Some("two".to_string()), result);
}

