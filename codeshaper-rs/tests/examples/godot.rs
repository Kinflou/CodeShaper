// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses
use crate::examples::{copy_directory, read};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::Outcome;


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
        "character_body_script.kdl".to_string(), "export_annotations.kdl".to_string()
    ])));
    assert_eq!(outcome("player.tscn"), Some(Outcome::Patched(vec!["character_body_scene.kdl".to_string()])));
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

    for (name, patch) in &operation.project.patches {
        let (backends, diagnostics) = operation.languages(&registry, patch, &solution.files());

        assert_eq!(backends.len(), 1, "{name} has no language");
        assert_eq!(diagnostics, vec![], "{name}");
    }

    assert_eq!(operation.check(&registry).unwrap(), vec![]);
}

#[test]
fn preview_matches_the_run() {
    let directory = tempfile::tempdir().unwrap();
    let example = directory.path();
    copy_directory(Path::new(EXAMPLE), example);

    let operation = Operation::from_file(&example.join("configuration.kdl")).unwrap();
    let preview = operation.session(&registry()).unwrap().preview().unwrap();
    let patched = preview.files.iter().map(|file| &file.path).collect::<Vec<_>>();
    let report = operation.run(&registry()).unwrap();

    let run = report.files.iter().filter(|file| file.outcome != Outcome::Unchanged).map(|file| &file.path).collect::<Vec<_>>();
    assert_eq!(patched, run);
    assert!(!patched.is_empty());

    for file in &preview.files {
        let written = [&operation.configuration.target, &operation.configuration.output].into_iter()
            .map(|root| example.join(root).join(&file.path))
            .filter_map(|path| fs::read_to_string(path).ok())
            .collect::<Vec<_>>();

        assert!(written.contains(&file.result), "{}", file.path.display());
    }
}
