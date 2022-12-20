// Standard Uses
use std::rc::Rc;

// Crate Uses

// External Uses
use codeshaper_general::shaping::operation::actions;
use codeshaper_general::shaping::operation::actions::composer;
use codeshaper_general::shaping::patch;


#[allow(unused)]
#[test]
fn maker_call() {
    let input = "Lets reverse the name John Doe?";

    let mut hello_maker_action = Rc::new(
        actions::maker::Maker::with(patch::maker::Maker {
            name: "hello_maker".to_string(),
            prepare: Some("Lets reverse the name (.*?) (.*?)?".to_string()),
            make: "We reversed to $2 $1".to_string(),
        })
    );

    let composer = composer::Composer::with_action_shared(hello_maker_action);

    // assert_eq!(hello_maker_action.make(), Some("Hello CodeShaper to the World!".to_string()))
}

