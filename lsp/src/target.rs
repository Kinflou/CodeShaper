// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses
use crate::preview::configuration;
use crate::syntax::Outline;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::{LocationMatches, Operation, Places};
use codeshaper::core::shaping::patch::Patch;


const ANY_FILE: &str = "**";


pub fn operation(project: &Path) -> Option<Operation> {
    Operation::from_file(&configuration(project).ok()?).ok()
}

pub fn patch(outline: &Outline) -> Patch {
    Patch {
        alias: outline.argument("alias").unwrap_or_default().to_string(),
        file: outline.argument("file").unwrap_or_default().to_string(),
        ..Patch::default()
    }
}

pub fn files(registry: &Registry, project: &Path, outline: &Outline) -> Option<(PathBuf, Vec<PathBuf>)> {
    let (directory, files) = operation(project)?.matching_files(registry, "", &patch(outline)).ok()?;

    Some((directory, files.into_iter().map(|file| file.path).collect()))
}

pub fn every_file(registry: &Registry, project: &Path) -> Vec<PathBuf> {
    let patch = Patch { file: ANY_FILE.to_string(), ..Patch::default() };

    operation(project)
        .and_then(|operation| operation.matching_files(registry, "", &patch).ok())
        .map(|(_, files)| files.into_iter().map(|file| file.path).collect())
        .unwrap_or_default()
}

pub fn matches(registry: &Registry, project: &Path, outline: &Outline, location: &str) -> Option<LocationMatches> {
    operation(project)?.locate(registry, "", &patch(outline), location).ok()
}

pub fn places(registry: &Registry, project: &Path, outline: &Outline, location: &str) -> Vec<Places> {
    operation(project)
        .and_then(|operation| operation.places(registry, "", &patch(outline), location).ok())
        .unwrap_or_default()
}
