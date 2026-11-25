// Standard Uses
use std::rc::Rc;

// Crate Uses
use crate::operation::registry;

// External Uses
use codeshaper_core::shaping::patch::Patch;
use codeshaper_core::shaping::patch::controller::PatchController;
use codeshaper_core::shaping::patch::validation::validate;


const CALLED: &str = r#"
    maker "inner" {
        make "<@[0]|@[1]>"
    }
    maker "prepared" {
        prepare r"(\w)(\w+)"
        make "$[1]@[0]$[0]"
    }
    switch "kind" {
        index "@[0]"
        cases one="1" two="2"
        default "none"
    }
    list "order" {
        index "@[1]"
        values "a" "b" "c"
    }
"#;

pub fn shaped(actions: &str, to: &str) -> String {
    let replacer = format!("replacer {{\nlocation \"Line\"\nfrom r\"(\\w+) (\\w+)\"\nto r##\"{to}\"##\n}}\n");
    let patch = Patch::from_kdl("", &format!("file \"*\"\nactions {{\n{actions}\n{replacer}}}\n")).unwrap();
    let registry = registry();
    let backend = registry.backend("lines").unwrap();

    assert!(validate(&patch, [backend], &[]).iter().all(|diagnostic| diagnostic.kind.code() == "unused-action"));

    let patch = Rc::new(patch);
    let result = PatchController::with(patch.clone()).apply(backend, "one two").unwrap();
    assert_eq!(PatchController::traced(patch, 9).apply(backend, "one two").unwrap(), result);

    result
}

fn passed(make: &str) -> String {
    shaped(&format!("{CALLED}\nmaker \"outer\" {{\nmake r##\"{make}\"##\n}}\n"), "#[outer]($[0], $[1])")
}


#[test]
fn makers_pass_their_arguments_on() {
    assert_eq!(passed("#[inner](@[0], @[1])"), "<one|two>");
    assert_eq!(passed("#[inner](@[1], @[0])"), "<two|one>");
    assert_eq!(passed("@[0] #[inner](\"x\", @[1]) @[1]"), "one <x|two> two");
    assert_eq!(passed("#[prepared](@[0])"), "neoneo");
    assert_eq!(passed("#[kind](@[1])"), "2");
    assert_eq!(passed("#[kind](@[0])#[kind](\"x\")"), "1none");
    assert_eq!(passed("#[order](@[0], \"2\")"), "c");
}

#[test]
fn arguments_reach_calls_inside_arguments() {
    assert_eq!(passed("#[inner](#[inner](@[0], @[1]), #[kind](@[0]))"), "<<one|two>|1>");
    assert_eq!(passed("#[inner](#[inner](#[inner](@[1], \"x\"), @[0]), @[1])"), "<<<two|x>|one>|two>");
}

#[test]
fn each_action_has_its_own_arguments() {
    let middle = "maker \"middle\" {\nmake \"#[inner](@[0], \\\"m\\\") @[0]\"\n}\n";
    let outer = "maker \"outer\" {\nmake \"#[middle](@[1]) @[0]\"\n}\n";

    assert_eq!(shaped(&format!("{CALLED}{middle}{outer}"), "#[outer]($[0], $[1])"), "<two|m> two one");
}

#[test]
fn switches_and_lists_pass_their_arguments_on() {
    let picking = r##"
        switch "picked" {
            index "#[kind](@[0])"
            cases "1"="first" "2"="second"
        }
        list "ordered" {
            index "#[kind](@[0])"
            values "a" "b" "c"
        }
    "##;

    assert_eq!(shaped(&format!("{CALLED}{picking}"), "#[picked]($[1]) #[ordered]($[0]) #[ordered]($[1])"), "second b c");
}
