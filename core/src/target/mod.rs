// Standard Uses
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use miette::Result;


pub type Options = BTreeMap<String, Vec<String>>;


#[derive(Debug, Clone, Default, PartialEq)]
pub struct Solution {
    pub name: String,
    pub directory: PathBuf,
    pub groups: Vec<Group>
}

impl Solution {
    pub fn files(&self) -> Vec<&File> {
        fn collect<'a>(groups: &'a [Group], files: &mut Vec<&'a File>) {
            for group in groups {
                files.extend(&group.files);
                collect(&group.groups, files);
            }
        }

        let mut files = vec![];
        collect(&self.groups, &mut files);

        let mut seen = HashSet::new();
        files.retain(|file| seen.insert(&file.path));

        files
    }

    pub fn retain(&mut self, keep: &dyn Fn(&File) -> bool) {
        fn filter(groups: &mut [Group], keep: &dyn Fn(&File) -> bool) {
            for group in groups {
                group.files.retain(|file| keep(file));
                filter(&mut group.groups, keep);
            }
        }

        filter(&mut self.groups, keep);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Group {
    pub name: String,
    pub groups: Vec<Group>,
    pub files: Vec<File>
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct File {
    pub path: PathBuf,
    pub language: Option<String>
}


pub trait TargetLoader {
    fn name(&self) -> &str;
    fn options(&self) -> &[&str] { &[] }
    fn load(&self, path: &Path, options: &Options) -> Result<Solution>;
}
