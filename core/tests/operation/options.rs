// Standard Uses
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// Crate Uses
use crate::operation::{read, registry, rename, workspace};

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::shaping::operation::configuration;
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::project::settings;
use codeshaper_core::shaping::target::Target;
use codeshaper_core::target::{File, Group, Options, Solution, TargetLoader};
use miette::Result;
use tempfile::TempDir;


struct DiskLoader(Rc<RefCell<Options>>);

impl TargetLoader for DiskLoader {
    fn name(&self) -> &str { "disk" }

    fn options(&self) -> &[&str] { &["files", "exclude"] }

    fn load(&self, path: &Path, options: &Options) -> Result<Solution> {
        fn walk(root: &Path, directory: &Path) -> Group {
            let mut group = Group { name: directory.display().to_string(), groups: vec![], files: vec![] };
            let mut entries: Vec<PathBuf> = fs::read_dir(directory).unwrap().map(|entry| entry.unwrap().path()).collect();
            entries.sort();

            for entry in entries {
                match entry.is_dir() {
                    true => group.groups.push(walk(root, &entry)),
                    false => group.files.push(File { path: entry.strip_prefix(root).unwrap().to_path_buf(), language: None })
                }
            }

            group
        }

        *self.0.borrow_mut() = options.clone();

        Ok(Solution { name: "Disk".to_string(), directory: path.to_path_buf(), groups: vec![walk(path, path)] })
    }
}


fn disk() -> (Registry, Rc<RefCell<Options>>) {
    let given = Rc::new(RefCell::new(Options::new()));
    let mut registry = registry();
    registry.add_loader(Box::new(DiskLoader(given.clone())));

    (registry, given)
}

fn options(given: &[(&str, &[&str])]) -> Options {
    given.iter()
        .map(|(name, values)| (name.to_string(), values.iter().map(|value| value.to_string()).collect()))
        .collect()
}

fn set(directory: &TempDir, path: &str, from: &str, to: &str) {
    let content = read(directory, path);
    assert!(content.contains(from), "{content}");
    fs::write(directory.path().join(path), content.replace(from, to)).unwrap();
}

fn on_disk(result: &str) -> TempDir {
    let directory = workspace(result, &[("rename.kdl", rename("lines", "Line"))]);
    set(&directory, "shaping/project.kdl", "\"list\"", "\"disk\"");

    directory
}

fn files(directory: &TempDir) -> Vec<String> {
    let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
    let solution = operation.target(&disk().0).unwrap();

    solution.files().iter().map(|file| file.path.to_string_lossy().replace('\\', "/")).collect()
}

fn decoded(target: &str) -> Result<Target, String> {
    let content = format!("name \"A\"\n{target}\nast-set \"cpp\"\ndescription \"\"\n");

    settings::from_kdl(&content)
        .map(|settings| settings.target)
        .map_err(|report| report.related().into_iter().flatten().map(|related| {
            let label = related.labels().into_iter().flatten().next().map(|label| content[label.offset()..label.offset() + label.len()].to_string());
            format!("{} | {related}", label.unwrap_or_default())
        }).collect::<Vec<_>>().join("\n"))
}


#[test]
fn lines_and_blocks_hold_the_same_entries() {
    let both = Target { value: "text".to_string(), options: options(&[("files", &["a.cpp", "src/b.cpp"])]) };

    for entries in [
        r#"files "a.cpp" "src/b.cpp""#,
        "files {\n        \"a.cpp\"\n        \"src/b.cpp\"\n    }",
        "files \"a.cpp\" {\n        \"src/b.cpp\"\n    }",
        "files {\n        a.cpp\n        /- \"gone.cpp\"\n        // \"c.cpp\"\n        \"src/b.cpp\"\n    }",
        "files \\\n        \"a.cpp\" \\\n        \"src/b.cpp\"",
        "files { \"a.cpp\"; \"src/b.cpp\"; }"
    ] {
        let target = format!("target \"text\" {{\n    {entries}\n}}");
        assert_eq!(decoded(&target).unwrap(), both, "{entries}");
        assert_eq!(decoded(&target.replace('\n', "\r\n")).unwrap(), both, "{entries}");
    }

    assert_eq!(decoded("target \"text\"").unwrap(), "text".into());
    assert_eq!(decoded("target \"text\" {\n}").unwrap(), "text".into());
    assert_eq!(decoded("target \"text\" {\n    files\n    exclude {\n    }\n}").unwrap().options, options(&[("files", &[]), ("exclude", &[])]));
}

#[test]
fn errors_point_at_what_is_wrong() {
    for (target, expected) in [
        ("target", "target | 'target' has no value"),
        ("target 1", "1 | expected text in quotes"),
        ("target \"a\" \"b\"", "\"b\" | 'target' takes one value, its options go in a block"),
        ("target \"a\" files=\"b\"", "files | 'target' takes no properties, its options go in a block"),
        ("target \"a\" {\n    files \"b\"\n    files \"c\"\n}", "files | 'files' is given twice"),
        ("target \"a\" {\n    files 1\n}", "1 | expected text in quotes"),
        ("target \"a\" {\n    files b=\"c\"\n}", "b | 'files' takes no properties"),
        ("target \"a\" {\n    files {\n        \"b\" \"c\"\n    }\n}", "\"c\" | a block takes one entry per line"),
        ("target \"a\" {\n    files {\n        \"b\" c=\"d\"\n    }\n}", "c | a block takes one entry per line"),
        ("target \"a\" {\n    files {\n        \"b\" { \"c\"; }\n    }\n}", "\"b\" | an entry takes no block")
    ] {
        assert_eq!(decoded(target).unwrap_err(), expected, "{target}");
    }
}

#[test]
fn configurations_hold_options_too() {
    let content = "name \"A\"\nproject \"shaping/\"\ntarget \"../engine/\" {\n    files \"core/*.cpp\"\n    exclude\n}\noutput \"out/\"\nresult \"replace\"\n";
    let configuration = configuration::from_kdl(content).unwrap();

    assert_eq!(configuration.target.value, "../engine/");
    assert_eq!(configuration.target.options, options(&[("files", &["core/*.cpp"]), ("exclude", &[])]));
}

#[test]
fn configurations_replace_the_defaults() {
    let defaults = "\"disk\" {\n    files \"a\" \"b\"\n    exclude \"c\"\n}";

    for (overrides, expected) in [
        ("", options(&[("files", &["a", "b"]), ("exclude", &["c"])])),
        (" {\n}", options(&[("files", &["a", "b"]), ("exclude", &["c"])])),
        (" {\n    files \"d\"\n}", options(&[("files", &["d"]), ("exclude", &["c"])])),
        (" {\n    files \"**/*\"\n}", options(&[("files", &["**/*"]), ("exclude", &["c"])])),
        (" {\n    exclude\n}", options(&[("files", &["a", "b"]), ("exclude", &[])])),
        (" {\n    exclude \"c\" \"d\"\n}", options(&[("files", &["a", "b"]), ("exclude", &["c", "d"])]))
    ] {
        let directory = on_disk("replace");
        set(&directory, "shaping/project.kdl", "\"disk\"", defaults);
        set(&directory, "configuration.kdl", "\"target/\"", &format!("\"target/\"{overrides}"));

        let (registry, given) = disk();
        Operation::from_file(&directory.path().join("configuration.kdl")).unwrap().target(&registry).unwrap();
        assert_eq!(*given.borrow(), expected, "{overrides}");
    }
}

#[test]
fn each_configuration_has_its_own_target() {
    let directory = on_disk("replace");
    let root = directory.path();
    fs::create_dir(root.join("engine")).unwrap();
    fs::write(root.join("engine/e.txt"), "old one\n").unwrap();
    fs::write(root.join("engine.kdl"), read(&directory, "configuration.kdl").replace("\"target/\"", "\"engine/\" {\n    files \"e.txt\"\n}")).unwrap();

    let found = Operation::configurations_for(&root.join("shaping")).unwrap();
    let names: Vec<&str> = found.iter().map(|path| path.file_name().unwrap().to_str().unwrap()).collect();
    assert_eq!(names, vec!["configuration.kdl", "engine.kdl"]);
    assert!(found.iter().all(|path| path.parent() == Some(root)), "{found:?}");
    assert_eq!(Operation::configuration_for(&root.join("shaping")).unwrap(), found[0]);

    let (registry, given) = disk();
    let solution = Operation::from_file(&found[1]).unwrap().target(&registry).unwrap();
    assert_eq!(solution.files(), vec![&File { path: "e.txt".into(), language: None }]);
    assert_eq!(*given.borrow(), options(&[("files", &["e.txt"])]));
}

#[test]
fn unknown_options_are_rejected() {
    let error = |directory: &TempDir| {
        let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
        operation.target(&disk().0).unwrap_err().to_string()
    };

    let directory = on_disk("replace");
    set(&directory, "shaping/project.kdl", "\"disk\"", "\"disk\" {\n    include \"a\"\n}");
    assert_eq!(error(&directory), "The target type 'disk' has no option 'include', its options are: files, exclude");

    let directory = workspace("replace", &[]);
    set(&directory, "configuration.kdl", "\"target/\"", "\"target/\" {\n    files \"a\"\n}");
    assert_eq!(error(&directory), "The target type 'list' takes no options, but has 'files'");
}

#[test]
fn own_folders_are_skipped() {
    let directory = on_disk("backup-and-replace");
    set(&directory, "configuration.kdl", "\"target/\"", "\"./\"");

    assert_eq!(files(&directory), vec!["configuration.kdl", "target/a.txt", "target/c.md", "target/nested/b.txt"]);

    for _ in 0..2 {
        let operation = Operation::from_file(&directory.path().join("configuration.kdl")).unwrap();
        operation.run(&disk().0).unwrap();
        fs::create_dir_all(directory.path().join("output")).unwrap();
        fs::write(directory.path().join("output/a.txt"), "old one\n").unwrap();
        assert_eq!(files(&directory), vec!["configuration.kdl", "target/a.txt", "target/c.md", "target/nested/b.txt"]);
    }

    assert_eq!(read(&directory, "target/a.txt"), "new one\nkeep\n");
    assert_eq!(read(&directory, "backup/target/a.txt"), "old one\nkeep\n");
    assert_eq!(read(&directory, "output/a.txt"), "old one\n");
}

#[test]
fn folders_around_the_target_are_not_skipped() {
    for output in ["target/", "./", "", "missing/"] {
        let directory = on_disk("replace");
        set(&directory, "configuration.kdl", "\"output/\"", &format!("\"{output}\""));

        assert_eq!(files(&directory), vec!["a.txt", "c.md", "nested/b.txt"], "{output}");
    }

    let directory = on_disk("replace");
    set(&directory, "configuration.kdl", "\"output/\"", "\"target/nested\"");
    assert_eq!(files(&directory), vec!["a.txt", "c.md"]);
}
