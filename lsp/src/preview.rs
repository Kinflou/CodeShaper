// Standard Uses
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::syntax::Outline;
use crate::values::prepared;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::operation::session::{Change, Preview};
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::trace::Trace;
use dunce::canonicalize;
use miette::{IntoDiagnostic, Result};


pub fn configuration(project: &Path) -> Result<PathBuf> {
    Operation::configuration_for(project)
}

pub fn preview(registry: &Registry, project: &Path, name: &str, text: &str) -> Result<(PathBuf, Vec<Change>)> {
    let patch = Patch::from_kdl(name, text)?;
    let operation = prepared(Operation::from_file(&configuration(project)?)?);
    let target = operation.directory.join(&operation.configuration.target.value);

    Ok((target, operation.preview(registry, name, &patch)?))
}

pub fn operation_preview(
    registry: &Registry, path: &Path, open: &HashMap<PathBuf, String>, keep_going: &dyn Fn() -> bool
) -> Result<Preview> {
    let read = |path: &Path| match canonicalize(path).ok().and_then(|path| open.get(&path)) {
        Some(text) => Ok(text.clone()),
        None => fs::read_to_string(path)
    };

    let configuration = configuration::from_kdl(&read(path).into_diagnostic()?)?;
    let operation = prepared(Operation::from_configuration_with(configuration, path.parent().unwrap_or(Path::new("")), &read)?);

    operation.session(registry)?.preview_while(keep_going)
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
