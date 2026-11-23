// Standard Uses
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

// Crate Uses
use crate::document::ast_set;
use crate::preview::configuration;
use crate::syntax::Outline;

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::{LocationMatches, Operation, Places};
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::project::ast_set::AstSet;


const ANY_FILE: &str = "**";

static LANGUAGES: Mutex<BTreeMap<(PathBuf, String), Vec<String>>> = Mutex::new(BTreeMap::new());
static FORGOTTEN: AtomicU64 = AtomicU64::new(0);


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

pub fn backends<'r>(registry: &'r Registry, project: &Path, outline: &Outline) -> Vec<&'r dyn LanguageBackend> {
    let patch = patch(outline);

    if !patch.alias.is_empty() {
        return registry.backend(&patch.alias).into_iter().collect()
    }

    let names = match ast_set(project) {
        Some(AstSet::Language(name)) => vec![name],
        Some(_) => matched_languages(registry, project, &patch),
        None => vec![]
    };

    names.iter().filter_map(|name| registry.backend(name)).collect()
}

fn matched_languages(registry: &Registry, project: &Path, patch: &Patch) -> Vec<String> {
    let key = (project.to_path_buf(), patch.file.clone());
    let asked = FORGOTTEN.load(Ordering::Acquire);
    if let Some(names) = languages().get(&key) { return names.clone() }

    let Some(operation) = operation(project) else { return vec![] };
    let Ok(solution) = operation.target(registry) else { return vec![] };
    let (backends, _) = operation.languages(registry, patch, &solution.files());
    let names: Vec<String> = backends.iter().map(|backend| backend.name().to_string()).collect();

    let mut known = languages();
    if FORGOTTEN.load(Ordering::Acquire) == asked {
        known.insert(key, names.clone());
    }

    names
}

fn languages() -> MutexGuard<'static, BTreeMap<(PathBuf, String), Vec<String>>> {
    LANGUAGES.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn forget_languages() {
    let mut known = languages();

    FORGOTTEN.fetch_add(1, Ordering::AcqRel);
    known.clear();
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
