// Standard Uses
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::{Actions, Patch};
use codeshaper_core::shaping::patch::maker::Maker;


#[allow(unused)]
#[test]
fn maker_call() {
    let input = "Lets reverse the name John and Doe?";

    let mut hello_maker_patch = Patch {
        enabled: false,
        alias: "".to_string(),
        file: "".to_string(),
        actions: Actions {
            builders: None,
            replacers: None,
            makers: Some(vec![
                Maker {
                    name: "hello_maker".to_owned(),
                    prepare: Some("Lets reverse the name (.+) and (.+)\\?".to_owned()),
                    make: "We reversed to $[1] and $[0]".to_owned(),
                }
            ]),
            switchers: None,
            lists: None
        },
    };
    let hello_maker_patch = Rc::new(hello_maker_patch);

    let mut patch_controller = PatchController::with(hello_maker_patch);
    let mut action = patch_controller
        .find_action("hello_maker", &patch_controller)
        .unwrap().maker().unwrap();

    let result = action.make(input, &patch_controller);

    assert_eq!(Some("We reversed to Doe and John".to_owned()), result)
}

