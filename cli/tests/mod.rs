// Standard Uses
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

// Crate Uses

// External Uses
use tempfile::TempDir;


const CPP14: &str = "../codeshaper-rs/examples/CPP14/Function Name Correction";
const GODOT: &str = "../codeshaper-rs/examples/Godot/Godot 3 to 4 Migration";

fn codeshaper(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codeshaper"))
        .args(arguments)
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

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

fn example(path: &str) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    copy_directory(Path::new(path), directory.path());

    directory
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

fn configuration(directory: &TempDir) -> String {
    directory.path().join("configuration.kdl").display().to_string()
}


#[test]
fn run_writes_results() {
    let directory = example(CPP14);
    let output = codeshaper(&["run", &configuration(&directory)]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("patched    CallerModule.cpp  (function_creation.kdl)"));
    assert!(stdout(&output).contains("2 patched, 2 unchanged"));

    for name in ["CallerModule.cpp", "ReturnModule.cpp"] {
        assert_eq!(
            read(&directory.path().join("target").join(name)),
            read(&directory.path().join("expected_result").join(name))
        );
    }
}

#[test]
fn dry_run_writes_nothing() {
    let directory = example(GODOT);
    let output = codeshaper(&["run", "--dry-run", &configuration(&directory)]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("-extends KinematicBody2D\n+extends CharacterBody2D"));
    assert!(stdout(&output).contains("2 file(s) would change, nothing was written"));
    assert_eq!(
        read(&directory.path().join("target/player.gd")),
        read(&Path::new(GODOT).join("target/player.gd"))
    );
    assert!(!directory.path().join("target_backup").exists());
}

#[test]
fn check_examples() {
    for path in [CPP14, GODOT] {
        let output = codeshaper(&["check", &Path::new(path).join("configuration.kdl").display().to_string()]);

        assert!(output.status.success(), "{path}: {}", stdout(&output));
        assert!(stdout(&output).contains("0 error(s), 0 warning(s)"));
    }
}

#[test]
fn check_broken_patch() {
    let directory = tempfile::tempdir().unwrap();
    let patch = directory.path().join("broken.kdl");
    fs::write(&patch, r##"enabled true
alias "cpp"
file "*.cpp"

actions {
    replacers {
        replacer {
            location "FunctionBody"
            from r"old ("
            to "#[missing]"
        }
    }
}
"##).unwrap();

    let output = codeshaper(&["check", "--patch", &patch.display().to_string()]);
    let text = stdout(&output);

    assert!(!output.status.success());
    assert!(text.contains("error[location-not-found] broken.kdl › replacer #1 › location"));
    assert!(text.contains("error[incorrect-regex] broken.kdl › replacer #1 › from"));
    assert!(text.contains("error[action-not-found] broken.kdl › replacer #1 › to"));
    assert!(text.contains("[broken.kdl:9:24]"));
    assert!(text.contains(r#"from r"old (""#));
    assert!(text.contains("3 error(s), 0 warning(s)"));
}

#[test]
fn apply_shows_diff_or_writes() {
    let directory = example(CPP14);
    let patch = directory.path().join("shaping_project/patches/function_return_creation.kdl").display().to_string();
    let file = directory.path().join("target/ReturnModule.cpp");
    let original = read(&file);

    let output = codeshaper(&["apply", &patch, &file.display().to_string()]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("+++ b/"));
    assert_eq!(read(&file), original);

    let output = codeshaper(&["apply", "--in-place", &patch, &file.display().to_string()]);
    assert!(output.status.success());
    assert_eq!(read(&file), read(&directory.path().join("expected_result/ReturnModule.cpp")));
}

#[test]
fn missing_configuration_fails_cleanly() {
    let output = codeshaper(&["run", "missing/configuration.kdl"]);
    let error = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(error.contains("Could not load operation configuration"));
    assert!(!error.contains("panicked"));
}
