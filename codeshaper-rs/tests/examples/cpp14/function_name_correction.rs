// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses
use crate::examples::{self, copy_directory, read};

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::{FileReport, Outcome};


const EXAMPLE: &str = "examples/CPP14/Function Name Correction/";
const EXAMPLE_TARGET: &str = "examples/CPP14/Function Name Correction/target/";


#[test]
fn target_files_have_a_language() {
    let registry = registry();
    let operation = Operation::from_file(&Path::new(EXAMPLE).join("configuration.kdl")).unwrap();
    let solution = operation.target(&registry).unwrap();

    assert_eq!(solution.files().len(), 4);

    for file in solution.files() {
        assert_eq!(operation.project.settings.ast_set.language(file), Some("cpp"), "{}", file.path.display());
    }
}

#[test]
fn runs_end_to_end() {
    let directory = tempfile::tempdir().unwrap();
    let example = directory.path();
    copy_directory(Path::new(EXAMPLE), example);

    let operation = Operation::from_file(&example.join("configuration.kdl")).unwrap();
    let mut files = operation.run(&registry()).unwrap().files;
    files.sort_by_key(|file| file.path.clone());

    let patched = |path: &str, patch: &str| FileReport {
        path: PathBuf::from(path), outcome: Outcome::Patched(vec![patch.to_string()])
    };
    let unchanged = |path: &str| FileReport { path: PathBuf::from(path), outcome: Outcome::Unchanged };

    assert_eq!(files, vec![
        patched("CallerModule.cpp", "function_creation.kdl"),
        unchanged("Module.cpp"),
        unchanged("Module.h"),
        patched("ReturnModule.cpp", "function_return_creation.kdl"),
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

    let index = |path: &str| session.files().iter().position(|(file, _)| *file == Path::new(path)).unwrap();
    let (caller, returning) = (index("CallerModule.cpp"), index("ReturnModule.cpp"));

    while session.next().is_some() {}

    assert_eq!(session.position(), 2);
    assert_eq!(session.content(caller).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE).join("expected_result/CallerModule.cpp")));
    assert_eq!(session.content(returning).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE).join("expected_result/ReturnModule.cpp")));

    while session.previous().is_some() {}

    assert_eq!(session.content(caller).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE_TARGET).join("CallerModule.cpp")));
    assert_eq!(session.content(returning).unwrap().replace("\r\n", "\n"), read(&Path::new(EXAMPLE_TARGET).join("ReturnModule.cpp")));
}

#[test]
fn preview_matches_the_run() {
    examples::preview_matches_the_run(EXAMPLE);
}
