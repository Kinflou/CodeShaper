// Standard Uses
use std::path::Path;

// Crate Uses
use crate::examples::{self, read, result_with};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;


const EXAMPLE: &str = "examples/Godot/Log Messages/";


#[test]
fn runs_end_to_end() {
    examples::runs_to_the_expected_result(EXAMPLE, &[
        ("player/player.gd", &["messages"]), ("project.godot", &["autoload"])
    ], &["door.gd", "log/game_log.gd"]);
}

#[test]
fn preview_matches_the_run() {
    examples::preview_matches_the_run(EXAMPLE);
}

#[test]
fn patches_validate() {
    let operation = Operation::from_file(&Path::new(EXAMPLE).join("configuration.kdl")).unwrap();

    assert_eq!(operation.check(&registry()).unwrap(), vec![]);
}

#[test]
fn both_patches_use_the_logger() {
    let expected = |path: &str| read(&Path::new(EXAMPLE).join("expected_result").join(path));

    for path in ["player/player.gd", "project.godot"] {
        assert_eq!(result_with(EXAMPLE, &[], path), expected(path));
        assert_eq!(result_with(EXAMPLE, &[("logger", "Journal")], path), expected(path).replace("GameLog", "Journal"));
    }
}

#[test]
fn the_level_has_a_default() {
    let expected = read(&Path::new(EXAMPLE).join("expected_result/player/player.gd"));

    assert_eq!(result_with(EXAMPLE, &[("level", "warn")], "player/player.gd"), expected.replace(".info(", ".warn("));
}
