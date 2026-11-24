// Standard Uses
use std::fs;
use std::path::PathBuf;

// Crate Uses

// External Uses
use codeshaper_lsp::preview::{configuration, preview};
use tempfile::TempDir;


pub const RENAME: &str = r#"enabled false
alias ""
file "*.cpp"

actions {
    replacer {
        location "FunctionDefinition"
        from r"old"
        to "new"
    }
}
"#;


pub fn operation() -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();

    let files = [
        ("configuration.kdl", "name \"Preview\"\nproject \"shaping_project/\"\ntarget \"target/\"\noutput \"out/\"\nresult \"replace\"\n"),
        ("other.kdl", "name \"Other\"\nproject \"elsewhere/\"\ntarget \"target/\"\noutput \"out/\"\nresult \"replace\"\n"),
        ("shaping_project/project.kdl", "name \"Preview\"\ntarget \"text\"\nast-set \"cpp\"\ndescription \"\"\n"),
        ("shaping_project/patches/rename.kdl", RENAME),
        ("target/a.cpp", "void old() {}\n"),
        ("target/b.cpp", "void keep() {}\n"),
    ];

    for (path, content) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    let project = root.join("shaping_project");
    (directory, project)
}


#[test]
fn the_configuration_pointing_at_the_project_is_found() {
    let (directory, project) = operation();

    let found = configuration(&project).unwrap().canonicalize().unwrap();
    assert_eq!(found, directory.path().canonicalize().unwrap().join("configuration.kdl"));
}

#[test]
fn preview_runs_the_buffer_on_matching_files() {
    let (directory, project) = operation();

    let (target, changes) = preview(&codeshaper::registry(), &project, "rename.kdl", RENAME).unwrap();
    let found: Vec<(String, String)> = changes.iter()
        .map(|change| (change.file.display().to_string(), change.result.as_ref().unwrap().clone()))
        .collect();

    assert_eq!(target.canonicalize().unwrap(), directory.path().join("target").canonicalize().unwrap());
    assert_eq!(found, vec![
        ("a.cpp".to_string(), "void new() {}\n".to_string()),
        ("b.cpp".to_string(), "void keep() {}\n".to_string()),
    ]);
}

#[test]
fn a_project_without_configuration_is_an_error() {
    let (directory, project) = operation();
    fs::remove_file(directory.path().join("configuration.kdl")).unwrap();

    let error = preview(&codeshaper::registry(), &project, "rename.kdl", RENAME).unwrap_err();

    assert!(error.to_string().starts_with("No operation configuration next to"), "{error}");
}
