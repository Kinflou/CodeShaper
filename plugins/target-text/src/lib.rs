// Relative Modules
pub mod project;

// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses
use crate::project::Project;

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{File, Group, Solution, TargetLoader};
use glob::{glob_with, MatchOptions, Pattern};
use miette::{bail, Context, IntoDiagnostic, Result};


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

            for path in included(group, directory)? {
                if files.iter().all(|file| file.path != path) {
                    files.push(File { path, language: None });
                }
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


fn options() -> MatchOptions {
    MatchOptions { require_literal_leading_dot: true, ..MatchOptions::new() }
}

fn included(group: &project::Group, directory: &Path) -> Result<Vec<PathBuf>> {
    let excludes = group.exclude.iter()
        .map(|pattern| Pattern::new(pattern).into_diagnostic()
            .wrap_err_with(|| format!("Group '{}' has an invalid exclude pattern '{}'", group.name, pattern)))
        .collect::<Result<Vec<_>>>()?;

    let mut paths = vec![];

    for pattern in &group.include {
        let base = Pattern::escape(&directory.to_string_lossy());
        let search = if base.is_empty() {
            pattern.clone()
        } else {
            format!("{}/{}", base.trim_end_matches('/'), pattern)
        };
        let entries = glob_with(&search, options())
            .into_diagnostic()
            .wrap_err_with(|| format!("Group '{}' has an invalid include pattern '{}'", group.name, pattern))?;

        let mut matched = false;

        for entry in entries {
            let path = entry.into_diagnostic()?;
            if !path.is_file() { continue }

            matched = true;
            let relative = path.strip_prefix(directory).into_diagnostic()?.to_path_buf();

            if !excludes.iter().any(|exclude| exclude.matches_path_with(&relative, options())) {
                paths.push(relative);
            }
        }

        if !matched {
            bail!("Group '{}' includes '{}', which matches no files in {}", group.name, pattern, directory.display());
        }
    }

    paths.sort();
    paths.dedup();

    Ok(paths)
}
