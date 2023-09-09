// Standard Uses
use std::collections::HashMap;

// Crate Uses

// External Uses
use codeshaper_core::shaping::operation::actions;
use codeshaper_core::shaping::patch;


#[test]
fn resolver_simple_switch() {
    let input = "2";

    let switch_resolver_action = actions::resolver::ResolverAction::with(patch::resolver::Resolver {
        name: "resolver_patch".to_string(),
        mode: "switch".to_string(),
        cases: HashMap::from([
            ("1".to_string(), "one".to_string()),
            ("2".to_string(), "two".to_string()),
            ("3".to_string(), "three".to_string()),
        ]),
        list: vec![],
        index: "$[0]".to_string(),
        default: "two".to_string(),
    });
    
    let output = switch_resolver_action.resolve(input);

    println!("{:#?}", output);
    assert_eq!(output, Some("two".to_string()));
}
