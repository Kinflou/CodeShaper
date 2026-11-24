// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper::core::shaping::project::ast_set::AstSet;
use codeshaper::core::shaping::project::{settings, Project};


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentKind {
    Patch { project: PathBuf },
    Settings,
    Configuration,
    Other
}

pub fn kind(path: &Path) -> DocumentKind {
    if path.extension().and_then(|extension| extension.to_str()) != Some("kdl") {
        return DocumentKind::Other
    }

    if let Some(project) = Project::containing(path) {
        return DocumentKind::Patch { project }
    }

    match path.file_name().and_then(|name| name.to_str()) {
        Some("settings.kdl") => DocumentKind::Settings,
        Some("configuration.kdl") => DocumentKind::Configuration,
        _ => DocumentKind::Other
    }
}

pub fn ast_set(project: &Path) -> Option<AstSet> {
    let content = fs::read_to_string(project.join("settings.kdl")).ok()?;

    settings::from_kdl(&content).ok().map(|settings| settings.ast_set)
}
