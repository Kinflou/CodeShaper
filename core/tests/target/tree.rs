// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper_core::target::{File, Group, Solution};


fn file(path: &str) -> File {
    File { path: path.into(), language: None }
}


#[test]
fn files_are_depth_first() {
    let solution = Solution {
        name: "Test Solution".to_string(),
        directory: PathBuf::new(),
        groups: vec![
            Group {
                name: "root".to_string(),
                groups: vec![
                    Group { name: "nested".to_string(), groups: vec![], files: vec![file("nested.cpp")] }
                ],
                files: vec![file("main.cpp")]
            },
            Group { name: "other".to_string(), groups: vec![], files: vec![file("other.cpp")] }
        ]
    };

    let paths: Vec<&Path> = solution.files().iter().map(|file| file.path.as_path()).collect();

    assert_eq!(paths, vec![Path::new("main.cpp"), Path::new("nested.cpp"), Path::new("other.cpp")]);
}

#[test]
fn files_in_several_groups_are_listed_once() {
    let group = |name: &str, files: Vec<File>, groups: Vec<Group>| Group { name: name.to_string(), groups, files };
    let typed = File { path: "main.cpp".into(), language: Some("cpp".to_string()) };
    let solution = Solution {
        name: "Test Solution".to_string(),
        directory: PathBuf::new(),
        groups: vec![
            group("root", vec![typed.clone(), file("main.cpp")], vec![group("nested", vec![file("main.cpp"), file("a.cpp")], vec![])]),
            group("other", vec![file("a.cpp"), file("b.cpp")], vec![])
        ]
    };

    assert_eq!(solution.files(), vec![&typed, &file("a.cpp"), &file("b.cpp")]);
}
