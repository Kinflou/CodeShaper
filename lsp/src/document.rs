// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};

// Crate Uses

// External Uses
use codeshaper::core::shaping::project::settings;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentKind {
    Patch { project: PathBuf },
    Settings,
    Configuration,
    TargetProject,
    Other
}

pub fn kind(path: &Path) -> DocumentKind {
    if path.extension().and_then(|extension| extension.to_str()) != Some("kdl") {
        return DocumentKind::Other
    }

    for ancestor in path.ancestors().skip(1) {
        if ancestor.file_name().and_then(|name| name.to_str()) != Some("patches") { continue }

        if let Some(project) = ancestor.parent().filter(|project| project.join("settings.kdl").is_file()) {
            return DocumentKind::Patch { project: project.to_path_buf() }
        }
    }

    match path.file_name().and_then(|name| name.to_str()) {
        Some("settings.kdl") => DocumentKind::Settings,
        Some("configuration.kdl") => DocumentKind::Configuration,
        Some("project.kdl") => DocumentKind::TargetProject,
        _ => DocumentKind::Other
    }
}

pub fn ast_set(project: &Path) -> String {
    fs::read_to_string(project.join("settings.kdl")).ok()
        .and_then(|content| settings::from_kdl(&content).ok())
        .map(|settings| settings.ast_set)
        .unwrap_or_default()
}
