// Standard Uses
use std::collections::HashMap;

// Crate Uses

// External Uses
use codeshaper_core::shaping::patch::resolver::Switch;
use codeshaper_core::shaping::operation::actions::indexer::SwitchAction;


#[test]
fn resolver_simple_switch() {
    let input = "2";

    let switch_action = SwitchAction::with_ref(Switch {
        name: "resolver_patch".to_string(),
        cases: HashMap::from([
            ("1".to_string(), "one".to_string()),
            ("2".to_string(), "two".to_string()),
            ("3".to_string(), "three".to_string()),
        ]),
        index: "$[0]".to_string(),
        default: Some("two".to_owned()),
    }, None);
    
    let output = switch_action.resolve_isolated(input);

    assert_eq!(output, Some("two".to_string()));
}
