// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::syntax::Outline;

// External Uses
use codeshaper::core::shaping::project::ast_set::AstSet;
use codeshaper::core::shaping::project::{settings, Project};


const CONFIGURED: [&str; 3] = ["project", "target", "result"];


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
        Some(settings::FILE) => DocumentKind::Settings,
        Some("configuration.kdl") => DocumentKind::Configuration,
        _ if is_configuration(path) => DocumentKind::Configuration,
        _ => DocumentKind::Other
    }
}

fn is_configuration(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else { return false };
    let outline = Outline::parse(&content);

    CONFIGURED.iter().all(|name| outline.top(name).is_some_and(|node| !node.arguments.is_empty()))
}

pub fn ast_set(project: &Path) -> Option<AstSet> {
    let content = fs::read_to_string(project.join(settings::FILE)).ok()?;

    settings::from_kdl(&content).ok().map(|settings| settings.ast_set)
}
