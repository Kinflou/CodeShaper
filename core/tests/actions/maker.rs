// Standard Uses
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::{Actions, Patch};


#[allow(unused)]
#[test]
fn maker_call() {
    let input = "Lets reverse the name John Doe?";

    let mut hello_maker_patch = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: None,
            replacers: None,
            makers: Some(vec![
                patch::maker::Maker {
                    name: "hello_maker".to_owned(),
                    prepare: Some("Lets reverse the name (.*?) and (.*?)?".to_owned()),
                    make: "We reversed to $[2] and $[1]".to_owned(),
                }
            ]),
            resolvers: None,
        },
    };
    let hello_maker_patch = Rc::new(hello_maker_patch);

    let mut patch_controller = PatchController::with_shared(hello_maker_patch);
    let mut action = patch_controller.find_action_mut("hello_maker")
        .unwrap().maker_mut().unwrap();

    assert_eq!(action.make(input), Some("We reversed to Doe and John".to_string()))
}

