// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses
use crate::examples::{copy_directory, read};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::Outcome;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::validation::{validate, Severity};


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
    let patches = Path::new(EXAMPLE).join("shaping_project/patches");

    for entry in fs::read_dir(patches).unwrap() {
        let path = entry.unwrap().path();
        let patch = Patch::from_path(&path).unwrap();

        let backend = match patch.alias.as_str() {
            "" => registry.backend_for(Path::new(&patch.file)),
            alias => registry.backend(alias)
        };

        let errors: Vec<_> = validate(&patch, backend).into_iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .collect();

        assert!(backend.is_some(), "{} has no backend", path.display());
        assert!(errors.is_empty(), "{}: {errors:#?}", path.display());
    }
}
