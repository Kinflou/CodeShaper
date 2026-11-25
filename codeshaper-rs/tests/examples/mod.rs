// Relative Modules
#[cfg(all(feature = "cpp", feature = "text"))]
mod cpp14;
#[cfg(all(feature = "gdscript", feature = "godot"))]
mod godot;

// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper::registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::{FileReport, Outcome};


pub fn copy_directory(from: &Path, to: &Path) {
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

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

pub fn runs_to_the_expected_result(example: &str, patched: &[(&str, &[&str])], unchanged: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_directory(Path::new(example), root);

    let mut files = Operation::from_file(&root.join("configuration.kdl")).unwrap().run(&registry()).unwrap().files;
    files.sort_by_key(|file| file.path.clone());

    let patched_files = patched.iter().map(|(path, patches)| FileReport {
        path: PathBuf::from(path), outcome: Outcome::Patched(patches.iter().map(|patch| patch.to_string()).collect())
    });
    let unchanged_files = unchanged.iter().map(|path| FileReport { path: PathBuf::from(path), outcome: Outcome::Unchanged });
    let mut expected: Vec<_> = patched_files.chain(unchanged_files).collect();
    expected.sort_by_key(|file| file.path.clone());
    assert_eq!(files, expected);

    let original = Path::new(example).join("target");
    for (path, _) in patched {
        assert_eq!(read(&root.join("target").join(path)), read(&root.join("expected_result").join(path)));
        assert_eq!(read(&root.join("target_backup").join(path)), read(&original.join(path)));
    }

    for path in unchanged {
        assert_eq!(read(&root.join("target").join(path)), read(&original.join(path)));
        assert!(!root.join("target_backup").join(path).exists());
    }
}

pub fn result_with(example: &str, values: &[(&str, &str)], file: &str) -> String {
    let mut operation = Operation::from_file(&Path::new(example).join("configuration.kdl")).unwrap();
    for (name, value) in values {
        operation.set(name, value).unwrap();
    }

    let registry = registry();
    let preview = operation.session(&registry).unwrap().preview().unwrap();
    let found = preview.files.iter().find(|preview| preview.path == Path::new(file)).unwrap();

    found.result.replace("\r\n", "\n")
}

pub fn preview_matches_the_run(example: &str) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    copy_directory(Path::new(example), root);

    let operation = Operation::from_file(&root.join("configuration.kdl")).unwrap();
    let preview = operation.session(&registry()).unwrap().preview().unwrap();
    let patched = preview.files.iter().map(|file| &file.path).collect::<Vec<_>>();
    let report = operation.run(&registry()).unwrap();

    let run = report.files.iter().filter(|file| file.outcome != Outcome::Unchanged).map(|file| &file.path).collect::<Vec<_>>();
    assert_eq!(patched, run);
    assert!(!patched.is_empty());

    for file in &preview.files {
        let written = [&operation.configuration.target.value, &operation.configuration.output].into_iter()
            .map(|folder| root.join(folder).join(&file.path))
            .filter_map(|path| fs::read_to_string(path).ok())
            .collect::<Vec<_>>();

        assert!(written.contains(&file.result), "{}", file.path.display());
    }
}
