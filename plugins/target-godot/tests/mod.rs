// Standard Uses
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{Group, Solution};


const PROJECT: &str = "tests/data/project/";

fn load(path: &str) -> miette::Result<Solution> {
    let mut registry = Registry::default();
    codeshaper_target_godot::register(&mut registry);

    registry.load_target("godot", Path::new(path))
}

fn tree(group: &Group, depth: usize, lines: &mut Vec<String>) {
    lines.push(format!("{}{}/", "  ".repeat(depth), group.name));

    for file in &group.files {
        let language = file.language.as_deref().unwrap_or("-");
        lines.push(format!("{}{} [{}]", "  ".repeat(depth + 1), file.path.display(), language));
    }

    for child in &group.groups {
        tree(child, depth + 1, lines);
    }
}


#[test]
fn loads_project_tree() {
    let solution = load(PROJECT).unwrap();

    assert_eq!(solution.name, "Fixture Game");
    assert_eq!(solution.directory, Path::new(PROJECT));

    let mut lines = vec![];
    tree(&solution.groups[0], 0, &mut lines);

    assert_eq!(lines, vec![
        "res:///",
        "  main.tscn [godot_resource]",
        "  project.godot [godot_resource]",
        "  player/",
        "    player/player.gd [gdscript]",
        "    player/player.tscn [godot_resource]",
        "  scripts/",
        "    scripts/Health.cs [c_sharp]",
        "  shaders/",
        "    shaders/water.gdshader [gdshader]",
    ]);
}

#[test]
fn skips_ignored_files() {
    let solution = load(PROJECT).unwrap();
    let paths: Vec<String> = solution.files().iter().map(|file| file.path.display().to_string()).collect();

    for skipped in [".godot/editor/cached.gd", "docs/draft.gd", "player/icon.svg.import", "player/icon.svg", "assets/readme.txt"] {
        assert!(!paths.contains(&skipped.to_string()), "{skipped} should be skipped");
    }
}

#[test]
fn name_falls_back_to_directory() {
    assert_eq!(load("tests/data/unnamed/").unwrap().name, "unnamed");
}

#[test]
fn requires_project_file() {
    let error = load("tests/data/").unwrap_err();

    assert!(error.to_string().contains("project.godot"));
}
