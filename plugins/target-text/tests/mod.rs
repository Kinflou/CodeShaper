// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{File, Options};
use codeshaper_target_text::options::listed;
use miette::Result;
use tempfile::TempDir;


const EXAMPLE_TARGET: &str = "../../codeshaper-rs/examples/CPP14/Function Name Correction/target/";

fn registry() -> Registry {
    let mut registry = Registry::default();
    codeshaper_target_text::register(&mut registry);

    registry
}

fn file(path: &str) -> File {
    File { path: path.into(), language: None }
}

fn options(given: &[(&str, &[&str])]) -> Options {
    given.iter()
        .map(|(name, values)| (name.to_string(), values.iter().map(|value| value.to_string()).collect()))
        .collect()
}

fn target(files: &[&str]) -> TempDir {
    let directory = tempfile::tempdir().unwrap();

    for file in files {
        let path = directory.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    directory
}

fn loaded(directory: &Path, given: &[(&str, &[&str])]) -> Result<Vec<String>> {
    let solution = registry().load_target("text", directory, &options(given))?;

    Ok(solution.files().iter().map(|file| file.path.to_string_lossy().replace('\\', "/")).collect())
}

fn failure(directory: &Path, given: &[(&str, &[&str])]) -> String {
    format!("{:?}", loaded(directory, given).unwrap_err())
}


#[test]
fn every_file_is_taken_by_default() {
    let solution = registry().load_target("text", Path::new(EXAMPLE_TARGET), &Options::new()).unwrap();

    assert_eq!(solution.name, "target");
    assert_eq!(solution.directory, Path::new(EXAMPLE_TARGET));
    assert_eq!(solution.groups.len(), 1);
    assert_eq!(solution.files(), vec![&file("CallerModule.cpp"), &file("Module.cpp"), &file("Module.h"), &file("ReturnModule.cpp")]);

    let directory = target(&["a.cpp", "src/b.cpp", ".hidden.cpp", ".git/c.cpp", "src/.cache/d.cpp"]);
    assert_eq!(loaded(directory.path(), &[]).unwrap(), vec!["a.cpp", "src/b.cpp"]);
    assert_eq!(loaded(directory.path(), &[("exclude", &["src/**"])]).unwrap(), vec!["a.cpp"]);
    assert_eq!(loaded(directory.path(), &[("exclude", &[])]).unwrap(), vec!["a.cpp", "src/b.cpp"]);
    assert_eq!(loaded(target(&[]).path(), &[]).unwrap(), Vec::<String>::new());
}

#[test]
fn missing_target() {
    assert!(failure(Path::new("tests/data/missing/"), &[]).contains("to be a folder"));
}

#[test]
fn missing_file() {
    let directory = target(&["a.cpp"]);

    assert!(failure(directory.path(), &[("files", &["a.cpp", "Missing.cpp"])]).contains("'Missing.cpp'"));
}

#[test]
fn patterns_and_exclude() {
    let files = loaded(Path::new("tests/data/globs/"), &[
        ("files", &["src/main.cpp", "**/*.cpp", "src/**/*.h"]), ("exclude", &["src/generated/**", "tests/**"])
    ]);

    assert_eq!(files.unwrap(), vec!["src/main.cpp", "src/util/math.cpp", "src/util/math.h"]);
}

#[test]
fn patterns_matching_nothing() {
    let directory = target(&["src/readme.txt"]);

    assert!(failure(directory.path(), &[("files", &["src/**/*.cpp"])]).contains("'src/**/*.cpp'"));
}

#[test]
fn files_takes_names_and_patterns() {
    let directory = target(&["a.cpp", "b.cpp", "src/c.cpp", "src/d.h", ".hidden.cpp"]);
    let path = directory.path();

    assert_eq!(loaded(path, &[("files", &["*"])]).unwrap(), vec!["a.cpp", "b.cpp"]);
    assert_eq!(loaded(path, &[("files", &["b.cpp", "**/*.cpp", "./b.cpp"])]).unwrap(), vec!["b.cpp", "a.cpp", "src/c.cpp"]);
    assert_eq!(loaded(path, &[("files", &["**/*"]), ("exclude", &["src/**", "a.cpp"])]).unwrap(), vec!["b.cpp"]);
    assert_eq!(loaded(path, &[("files", &["a.cpp", "b.cpp"]), ("exclude", &["a*"])]).unwrap(), vec!["b.cpp"]);
    assert_eq!(loaded(path, &[("files", &[".hidden.cpp"])]).unwrap(), vec![".hidden.cpp"]);

    for names in [&["*.rs"], &["src"], &["["]] {
        assert!(loaded(path, &[("files", names)]).is_err(), "{names:?}");
    }
    assert!(failure(path, &[("exclude", &["["])]).contains("'exclude' has an invalid pattern '['"));
    assert!(failure(path, &[("files", &[])]).contains("'files' lists nothing"));
}

#[test]
fn names_with_brackets_are_exact() {
    let directory = target(&["[id].tsx", "i.tsx"]);

    assert_eq!(loaded(directory.path(), &[("files", &["[id].tsx"])]).unwrap(), vec!["[id].tsx"]);
    assert_eq!(loaded(directory.path(), &[("files", &["[i].tsx"])]).unwrap(), vec!["i.tsx"]);
}

#[test]
fn unknown_options_are_rejected() {
    let error = loaded(target(&["a.cpp"]).path(), &[("include", &["*.cpp"])]).unwrap_err();

    assert_eq!(error.to_string(), "The target type 'text' has no option 'include', its options are: files, exclude");
}

#[test]
fn listed_entries_have_their_range() {
    let content = "name \"A\"\ntarget \"text\" {\n    files \"é.cpp\" {\n        \"b.cpp\"\n        /- \"gone.cpp\"\n        c.cpp\n    }\n    other \"x\"\n    exclude {\n        \"x/**\"\n    }\n}\nfiles \"outside\"\n";

    for content in [content.to_string(), content.replace('\n', "\r\n")] {
        let found: Vec<(&str, String, &str)> = listed(&content).unwrap().iter()
            .map(|entry| (entry.field.name(), entry.value.clone(), &content[entry.range.clone()]))
            .collect();

        assert_eq!(found, vec![
            ("files", "é.cpp".to_string(), "\"é.cpp\""), ("files", "b.cpp".to_string(), "\"b.cpp\""),
            ("files", "c.cpp".to_string(), "c.cpp"), ("exclude", "x/**".to_string(), "\"x/**\"")
        ]);
    }
}
