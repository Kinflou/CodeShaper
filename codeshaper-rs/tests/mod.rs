#![cfg(all(feature = "cpp", feature = "text"))]

// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::{FileReport, Outcome};


const EXAMPLE: &str = "examples/CPP14/Function Name Correction/";
const EXAMPLE_TARGET: &str = "examples/CPP14/Function Name Correction/target/";

fn copy_directory(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();

    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let destination = to.join(path.file_name().unwrap());

        if path.is_dir() {
            copy_directory(&path, &destination);
        } else {
            fs::copy(&path, &destination).unwrap();
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}


#[test]
fn default_plugins_registered() {
    let registry = registry();

    assert_eq!(registry.backend("cpp").unwrap().name(), "cpp");
    assert_eq!(registry.loader("text").unwrap().name(), "text");
}

#[cfg(all(feature = "gdscript", feature = "godot"))]
#[test]
fn godot_plugins_registered() {
    let registry = registry();

    assert_eq!(registry.backend("gdscript").unwrap().name(), "gdscript");
    assert_eq!(registry.backend("godot_resource").unwrap().name(), "godot_resource");
    assert_eq!(registry.loader("godot").unwrap().name(), "godot");
}

#[test]
fn example_target_files_have_backends() {
    let registry = registry();
    let solution = registry.load_target("text", Path::new(EXAMPLE_TARGET)).unwrap();

    assert_eq!(solution.files().len(), 4);

    for file in solution.files() {
        let backend = registry.backend_for(&file.path).unwrap();

        assert_eq!(backend.name(), "cpp", "{} has no cpp backend", file.path.display());
    }
}

#[test]
fn example_runs_end_to_end() {
    let directory = tempfile::tempdir().unwrap();
    let example = directory.path();
    copy_directory(Path::new(EXAMPLE), example);

    let operation = Operation::from_file(&example.join("configuration.kdl")).unwrap();
    let report = operation.run(&registry()).unwrap();

    let patched = |path: &str, patch: &str| FileReport {
        path: PathBuf::from(path), outcome: Outcome::Patched(vec![patch.to_string()])
    };
    let unchanged = |path: &str| FileReport { path: PathBuf::from(path), outcome: Outcome::Unchanged };

    assert_eq!(report.files, vec![
        patched("CallerModule.cpp", "function_creation.kdl"),
        patched("ReturnModule.cpp", "function_return_creation.kdl"),
        unchanged("Module.cpp"),
        unchanged("Module.h"),
    ]);

    for name in ["CallerModule.cpp", "ReturnModule.cpp"] {
        assert_eq!(read(&example.join("target").join(name)), read(&example.join("expected_result").join(name)));
        assert_eq!(read(&example.join("target_backup").join(name)), read(&Path::new(EXAMPLE_TARGET).join(name)));
    }

    for name in ["Module.cpp", "Module.h"] {
        assert_eq!(read(&example.join("target").join(name)), read(&Path::new(EXAMPLE_TARGET).join(name)));
        assert!(!example.join("target_backup").join(name).exists());
    }

    assert!(!example.join("target_result").exists());
}

#[test]
fn example_session_walks_back_to_original() {
    let operation = Operation::from_file(&Path::new(EXAMPLE).join("configuration.kdl")).unwrap();
    let registry = registry();
    let mut session = operation.session(&registry).unwrap();

    while session.next().is_some() {}

    assert_eq!(session.position(), 2);
    assert_eq!(session.content(0).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE).join("expected_result/CallerModule.cpp")));
    assert_eq!(session.content(1).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE).join("expected_result/ReturnModule.cpp")));

    while session.previous().is_some() {}

    assert_eq!(session.content(0).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE_TARGET).join("CallerModule.cpp")));
    assert_eq!(session.content(1).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE_TARGET).join("ReturnModule.cpp")));
}

#[cfg(all(feature = "gdscript", feature = "godot"))]
#[test]
fn godot_migration_runs_end_to_end() {
    const GODOT_EXAMPLE: &str = "examples/Godot/Godot 3 to 4 Migration/";

    let directory = tempfile::tempdir().unwrap();
    let example = directory.path();
    copy_directory(Path::new(GODOT_EXAMPLE), example);

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

    let original = Path::new(GODOT_EXAMPLE).join("target");
    for name in ["project.godot", "notes/draft.gd", "icon.svg.import"] {
        assert_eq!(read(&example.join("target").join(name)), read(&original.join(name)));
    }
}
