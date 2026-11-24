// Relative Modules
mod actions;
mod broken;
mod check;
mod languages;
mod lines;
mod located;
mod options;
mod places;
mod preview;
mod run;
mod session;
mod trace;

// Standard Uses
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::LanguageBackend;
use codeshaper_core::registry::Registry;
use codeshaper_core::shaping::operation::Operation;
use codeshaper_core::shaping::operation::report::{FileReport, Outcome};
use codeshaper_core::target::{File, Group, Options, Solution, TargetLoader};
use miette::{bail, Result};
use tempfile::TempDir;


pub struct LineBackend;

impl LanguageBackend for LineBackend {
    fn name(&self) -> &str { "lines" }

    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>> {
        if location == "File" {
            return Ok(vec![Range { start: 0, end: source.len() }])
        }
        if location != "Line" {
            bail!("Unknown location {location}")
        }

        let mut ranges = vec![];
        let mut start = 0;

        for line in source.split_inclusive('\n') {
            ranges.push(start..start + line.trim_end_matches('\n').len());
            start += line.len();
        }

        Ok(ranges)
    }
}

pub struct ListLoader;

impl TargetLoader for ListLoader {
    fn name(&self) -> &str { "list" }

    fn load(&self, path: &Path, _options: &Options) -> Result<Solution> {
        let files = ["a.txt", "nested/b.txt", "c.md"].iter()
            .map(|path| File { path: path.into(), language: None })
            .collect();

        Ok(Solution {
            name: "List".to_string(),
            directory: path.to_path_buf(),
            groups: vec![Group { name: "root".to_string(), groups: vec![], files }]
        })
    }
}

pub fn registry() -> Registry {
    let mut registry = Registry::default();
    registry.add_backend(Box::new(LineBackend));
    registry.add_loader(Box::new(ListLoader));

    registry
}

pub fn patch(alias: &str, location: &str, from: &str, to: &str) -> String {
    format!(r#"enabled true
alias "{alias}"
file "**/*.txt"

actions {{
    replacer {{
        location "{location}"
        from r"{from}"
        to "{to}"
    }}
}}
"#)
}

pub fn rename(alias: &str, location: &str) -> String {
    patch(alias, location, r"old (\w+)", "new $[0]")
}

pub fn workspace(result: &str, patches: &[(&str, String)]) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();

    let files = [
        ("configuration.kdl", format!(
            "name \"Run\"\nproject \"shaping/\"\ntarget \"target/\"\noutput \"output/\"\nbackup \"backup/\"\nresult \"{result}\"\n"
        )),
        ("shaping/project.kdl", "name \"Run\"\ntarget \"list\"\nast-set \"lines\"\ndescription \"\"\n".to_string()),
        ("target/a.txt", "old one\nkeep\n".to_string()),
        ("target/nested/b.txt", "keep\n".to_string()),
        ("target/c.md", "old three\n".to_string()),
    ];

    let patches = patches.iter().map(|(name, patch)| (format!("shaping/patches/{name}"), patch.clone()));
    let files = files.into_iter().map(|(path, content)| (path.to_string(), content));

    for (path, content) in files.chain(patches) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    directory
}

pub fn run(directory: &TempDir) -> Result<Vec<FileReport>> {
    let operation = Operation::from_file(&directory.path().join("configuration.kdl"))?;

    Ok(operation.run(&registry())?.files)
}

pub fn read(directory: &TempDir, path: &str) -> String {
    fs::read_to_string(directory.path().join(path)).unwrap()
}

pub fn report(path: &str, outcome: Outcome) -> FileReport {
    FileReport { path: PathBuf::from(path), outcome }
}
