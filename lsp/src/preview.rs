// Standard Uses
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::syntax::Outline;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::operation::session::Change;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::trace::Trace;
use dunce::canonicalize;
use miette::{miette, IntoDiagnostic, Result};


pub fn configuration(project: &Path) -> Result<PathBuf> {
    let project = canonicalize(project).into_diagnostic()?;
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
            canonicalize(parent.join(found.project)).is_ok_and(|directory| directory == project)
        })
        .ok_or_else(|| miette!("No operation configuration next to {} points at it", project.display()))
}

pub fn preview(registry: &Registry, project: &Path, name: &str, text: &str) -> Result<(PathBuf, Vec<Change>)> {
    let patch = Patch::from_kdl(name, text)?;
    let operation = Operation::from_file(&configuration(project)?)?;
    let target = operation.directory.join(&operation.configuration.target);

    Ok((target, operation.preview(registry, name, &patch)?))
}

pub fn replacement_counts(text: &str, trace: &Trace) -> Vec<(Range<usize>, String)> {
    let outline = Outline::parse(text);

    outline.nodes.iter().enumerate()
        .filter(|(_, node)| node.name == "replacer")
        .filter_map(|(index, node)| {
            let action = trace.action(&outline.action_path(index)?).cloned().unwrap_or_default();
            let title = match action.matches {
                0 => "No replacements".to_string(),
                matches => format!("{matches} replacement(s) in {} file(s)", action.matched_files)
            };

            Some((node.name_range.clone(), title))
        })
        .collect()
}
