// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::operation::session::Change;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::patch::Patch;
use miette::{miette, IntoDiagnostic, Result};


pub fn configuration(project: &Path) -> Result<PathBuf> {
    let project = project.canonicalize().into_diagnostic()?;
    let parent = project.parent().ok_or_else(|| miette!("{} has no parent directory", project.display()))?;

    let mut candidates: Vec<PathBuf> = fs::read_dir(parent).into_diagnostic()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension == "kdl"))
        .collect();
    candidates.sort();

    candidates.into_iter()
        .find(|path| {
            let Some(found) = fs::read_to_string(path).ok().and_then(|content| configuration::from_kdl(&content).ok()) else {
                return false
            };
            parent.join(found.project).canonicalize().is_ok_and(|directory| directory == project)
        })
        .ok_or_else(|| miette!("No operation configuration next to {} points at it", project.display()))
}

pub fn preview(registry: &Registry, project: &Path, name: &str, text: &str) -> Result<(PathBuf, Vec<Change>)> {
    let patch = Patch::from_kdl(name, text)?;
    let operation = Operation::from_file(&configuration(project)?)?;
    let target = operation.directory.join(&operation.configuration.target);

    Ok((target, operation.preview(registry, name, &patch)?))
}
