// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper_core::target::{File, Group, Solution, TargetLoader};
use codeshaper_core::target::text::TextLoader;


const EXAMPLE_TARGET: &str = "../codeshaper-rs/examples/CPP14/Function Name Correction/target/";

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
fn text_loader_loads_example() {
    let solution = TextLoader.load(Path::new(EXAMPLE_TARGET)).unwrap();

    assert_eq!(solution.name, "Function Name Correction");
    assert_eq!(solution.directory, Path::new(EXAMPLE_TARGET));
    assert_eq!(solution.groups.len(), 1);
    assert_eq!(solution.groups[0].name, "root");
    assert_eq!(
        solution.files(),
        vec![&file("CallerModule.cpp"), &file("ReturnModule.cpp"), &file("Module.cpp"), &file("Module.h")]
    );
}
