// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses
use crate::examples::{copy_directory, read};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::{FileReport, Outcome};


const EXAMPLE: &str = "examples/CPP14/Function Name Correction/";
const EXAMPLE_TARGET: &str = "examples/CPP14/Function Name Correction/target/";


#[test]
fn target_files_have_backends() {
    let registry = registry();
    let solution = registry.load_target("text", Path::new(EXAMPLE_TARGET)).unwrap();

    assert_eq!(solution.files().len(), 4);

    for file in solution.files() {
        let backend = registry.backend_for(&file.path).unwrap();

        assert_eq!(backend.name(), "cpp", "{} has no cpp backend", file.path.display());
    }
}

#[test]
fn runs_end_to_end() {
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
fn session_walks_back_to_original() {
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
