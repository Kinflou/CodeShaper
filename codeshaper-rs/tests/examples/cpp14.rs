// Standard Uses
use std::fs;
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
