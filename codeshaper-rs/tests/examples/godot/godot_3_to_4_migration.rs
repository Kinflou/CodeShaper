// Standard Uses
use std::path::Path;

// Crate Uses
use crate::examples::{self, copy_directory, read};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::Outcome;
use codeshaper::core::shaping::project::ProjectPatch;


const EXAMPLE: &str = "examples/Godot/Godot 3 to 4 Migration/";


#[test]
fn runs_end_to_end() {
    let directory = tempfile::tempdir().unwrap();
    let example = directory.path();
    copy_directory(Path::new(EXAMPLE), example);

    let operation = Operation::from_file(&example.join("configuration.kdl")).unwrap();
    let report = operation.run(&registry()).unwrap();

    let outcome = |path: &str| report.files.iter()
        .find(|file| file.path == Path::new(path))
        .map(|file| file.outcome.clone());

    assert_eq!(outcome("player.gd"), Some(Outcome::Patched(vec![
        "body_script".to_string(), "export_annotations.kdl".to_string()
    ])));
    assert_eq!(outcome("player.tscn"), Some(Outcome::Patched(vec!["body_scene".to_string()])));
    assert_eq!(outcome("project.godot"), Some(Outcome::Unchanged));
    assert_eq!(outcome("notes/draft.gd"), None);
    assert_eq!(report.files.len(), 3);

    for name in ["player.gd", "player.tscn"] {
        assert_eq!(read(&example.join("target").join(name)), read(&example.join("expected_result").join(name)));
    }

    let original = Path::new(EXAMPLE).join("target");
    for name in ["project.godot", "notes/draft.gd", "icon.svg.import"] {
        assert_eq!(read(&example.join("target").join(name)), read(&original.join(name)));
    }
}

#[test]
fn patches_validate() {
    let registry = registry();
    let operation = Operation::from_file(&Path::new(EXAMPLE).join("configuration.kdl")).unwrap();
    let solution = operation.target(&registry).unwrap();

    for ProjectPatch { name, patch, .. } in &operation.project.patches {
        let (backends, diagnostics) = operation.languages(&registry, patch, &solution.files());

        assert_eq!(backends.len(), 1, "{name} has no language");
        assert_eq!(diagnostics, vec![], "{name}");
    }

    assert_eq!(operation.check(&registry).unwrap(), vec![]);
}

#[test]
fn preview_matches_the_run() {
    examples::preview_matches_the_run(EXAMPLE);
}
