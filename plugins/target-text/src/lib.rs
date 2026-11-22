// Relative Modules
pub mod project;

// Standard Uses
use std::path::Path;

// Crate Uses
use crate::project::Project;

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{File, Group, Solution, TargetLoader};
use miette::{bail, Context, Result};


pub fn register(registry: &mut Registry) {
    registry.add_loader(Box::new(TextLoader));
}


pub struct TextLoader;

impl TargetLoader for TextLoader {
    fn name(&self) -> &str { "text" }

    fn load(&self, path: &Path) -> Result<Solution> {
        let project = Project::from_path(path)?;

        fn group_recursive(group: &project::Group, directory: &Path, project: &str) -> Result<Group> {
            let mut files = vec![];

            for file in &group.files {
                if !directory.join(file).is_file() {
                    bail!("Target project '{}' lists '{}', which does not exist in {}",
                        project, file, directory.display());
                }

                files.push(File { path: file.into(), language: None });
            }

            let groups = group.groups.iter()
                .map(|child| group_recursive(child, directory, project))
                .collect::<Result<Vec<_>>>()?;

            Ok(Group { name: group.name.clone(), groups, files })
        }

        let groups = project.groups.iter()
            .map(|group| group_recursive(group, path, &project.name))
            .collect::<Result<Vec<_>>>()
            .wrap_err_with(|| format!("Could not load target project at {}", path.display()))?;

        Ok(Solution { name: project.name, directory: path.to_path_buf(), groups })
    }
}
