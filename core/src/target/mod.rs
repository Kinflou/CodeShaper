// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use miette::Result;


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

        files
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
    fn load(&self, path: &Path) -> Result<Solution>;
}
