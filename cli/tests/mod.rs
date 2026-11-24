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
    replacer {
        location "FunctionBody"
        from r"old ("
        to "#[missing]"
    }
}
"##).unwrap();

    let output = codeshaper(&["check", "--patch", &patch.display().to_string()]);
    let text = stdout(&output);

    assert!(!output.status.success());
    assert!(text.contains("error[location-not-found] broken.kdl › replacer #1 › location"));
    assert!(text.contains("error[incorrect-regex] broken.kdl › replacer #1 › from"));
    assert!(text.contains("error[action-not-found] broken.kdl › replacer #1 › to"));
    assert!(text.contains("[broken.kdl:8:20]"));
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

#[test]
fn apply_uses_the_projects_language() {
    let directory = example(CPP14);
    let patch = directory.path().join("shaping_project/patches/function_return_creation.kdl");
    let file = directory.path().join("target/ReturnModule.cpp");
    let renamed = directory.path().join("target/ReturnModule.unknown");
    fs::copy(&file, &renamed).unwrap();
    let apply = |patch: &Path, file: &Path, extra: &[&str]| {
        let output = codeshaper(&[&["apply", &patch.display().to_string(), &file.display().to_string()], extra].concat());
        (output.status.success(), stdout(&output), String::from_utf8_lossy(&output.stderr).to_string())
    };

    assert!(apply(&patch, &renamed, &[]).1.contains("+++ b/"));
    assert!(apply(&patch, &file, &["--language", "cpp"]).0);
    assert!(apply(&patch, &file, &["--language", "cobol"]).2.contains("'cobol' is not a known language"));

    let alone = directory.path().join("alone.kdl");
    fs::copy(&patch, &alone).unwrap();
    assert!(apply(&alone, &file, &[]).2.contains("use --language"));
    assert!(apply(&alone, &file, &["--language", "cpp"]).1.contains("+++ b/"));

    let settings = directory.path().join("shaping_project/project.kdl");
    let content = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, content.replace("ast-set \"cpp\"", "ast-set { cpp \"Return*.cpp\"; }")).unwrap();
    assert!(apply(&patch, &file, &[]).1.contains("+++ b/"));
    assert!(apply(&patch, &directory.path().join("target/Module.cpp"), &[]).2.contains("has no language"));
    assert!(apply(&patch, &directory.path().join("expected_result/ReturnModule.cpp"), &[]).2.contains("is not in the project's target"));

    fs::write(&alone, format!("alias \"cpp\"\n{}", fs::read_to_string(&alone).unwrap())).unwrap();
    assert!(apply(&alone, &file, &[]).1.contains("+++ b/"));
    assert!(apply(&alone, &file, &["--language", "gdscript"]).2.contains("names the language 'cpp', not 'gdscript'"));
}

#[test]
fn check_says_when_locations_were_not_checked() {
    let directory = tempfile::tempdir().unwrap();
    let patch = directory.path().join("alone.kdl");
    fs::write(&patch, "enabled true\nfile \"a\"\nactions {\n    replacer {\n        location \"Nothing\"\n        from \"a\"\n        to \"b\"\n    }\n}\n").unwrap();
    let path = patch.display().to_string();

    let output = codeshaper(&["check", "--patch", &path]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("weren't checked"));

    let output = codeshaper(&["check", "--patch", &path, "--language", "cpp"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("error[location-not-found]"));
}

fn block(name: &str, from: &str, to: &str) -> String {
    format!(
        "patch \"{name}\" {{\n    enabled true\n    alias \"cpp\"\n    file \"*\"\n    actions {{\n        \
        replacer {{\n            location \"Identifier\"\n            from r\"{from}\"\n            to \"{to}\"\n        }}\n    }}\n}}\n"
    )
}

#[test]
fn apply_takes_one_of_several() {
    let directory = tempfile::tempdir().unwrap();
    let patch = directory.path().join("several.kdl");
    let file = directory.path().join("a.cpp");
    fs::write(&patch, block("one", "a", "b") + &block("two", "a", "c")).unwrap();
    fs::write(&file, "int a;\n").unwrap();
    let apply = |extra: &[&str]| {
        let output = codeshaper(&[&["apply", &patch.display().to_string(), &file.display().to_string()], extra].concat());
        (output.status.success(), stdout(&output), String::from_utf8_lossy(&output.stderr).to_string())
    };

    assert!(apply(&["--name", "one"]).1.contains("+int b;"));
    assert!(apply(&["--name", "two"]).1.contains("+int c;"));

    let (success, _, error) = apply(&[]);
    assert!(!success && error.contains("has several patches, name one with --name: one, two"), "{error}");
    let (success, _, error) = apply(&["--name", "three"]);
    assert!(!success && error.contains("has no patch named 'three', only: one, two"), "{error}");

    fs::write(&patch, block("one", "a", "b")).unwrap();
    assert!(apply(&[]).1.contains("+int b;"));
    assert!(apply(&["--name", "one"]).1.contains("+int b;"));

    fs::write(&patch, block("one", "a", "#[missing]")).unwrap();
    assert!(apply(&[]).1.contains("error[action-not-found] one › replacer #1 › to"));
}

#[test]
fn check_reports_every_patch_of_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let (first, second) = (directory.path().join("first.kdl"), directory.path().join("second.kdl"));
    fs::write(&first, block("one", "a", "#[lost]") + &block("two", "(", "b")).unwrap();
    fs::write(&second, format!("\n\n{}", block("one", "a", "#[gone]"))).unwrap();

    let output = codeshaper(&["check", "--patch", &first.display().to_string(), &second.display().to_string()]);
    let text = stdout(&output);

    assert!(!output.status.success());
    assert!(text.contains("error[action-not-found] one › replacer #1 › to"));
    assert!(text.contains("error[incorrect-regex] two › replacer #1 › from"));
    assert!(text.contains("[first.kdl:9:17]"), "{text}");
    assert!(text.contains("[first.kdl:20:"), "{text}");
    assert!(text.contains("[second.kdl:11:17]"), "{text}");
    assert!(text.contains("3 error(s), 0 warning(s)"));
}
