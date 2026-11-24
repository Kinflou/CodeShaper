// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{File, Group, Options, Solution, TargetLoader};
use miette::{bail, Context, IntoDiagnostic, Result};


pub fn register(registry: &mut Registry) {
    registry.add_loader(Box::new(GodotLoader));
}


pub struct GodotLoader;

impl TargetLoader for GodotLoader {
    fn name(&self) -> &str { "godot" }

    fn load(&self, path: &Path, _options: &Options) -> Result<Solution> {
        let project = path.join("project.godot");

        if !project.is_file() {
            bail!("Expected a Godot project with a project.godot at {}", path.display());
        }

        let content = fs::read_to_string(&project)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not read {}", project.display()))?;

        let name = project_name(&content)
            .or_else(|| path.canonicalize().ok()?.file_name()?.to_str().map(str::to_string))
            .unwrap_or_default();

        let root = collect(path, path, "res://".to_string())?;

        Ok(Solution { name, directory: path.to_path_buf(), groups: vec![root] })
    }
}


fn project_name(content: &str) -> Option<String> {
    content.lines()
        .find_map(|line| line.trim().strip_prefix("config/name="))
        .map(|value| value.trim().trim_matches('"').to_string())
}

fn language(path: &Path) -> Option<&'static str> {
    if path.file_name()? == "project.godot" {
        return Some("godot_resource")
    }

    match path.extension()?.to_str()? {
        "gd" => Some("gdscript"),
        "tscn" | "tres" => Some("godot_resource"),
        "gdshader" => Some("gdshader"),
        "cs" => Some("c_sharp"),
        _ => None
    }
}

fn collect(root: &Path, directory: &Path, name: String) -> Result<Group> {
    let mut entries = fs::read_dir(directory)
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not read {}", directory.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .into_diagnostic()?;
    entries.sort();

    let mut group = Group { name, groups: vec![], files: vec![] };

    for path in entries {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if path.is_dir() {
            if file_name.starts_with('.') || path.join(".gdignore").exists() { continue }

            let child = collect(root, &path, file_name)?;

            if !child.files.is_empty() || !child.groups.is_empty() {
                group.groups.push(child);
            }
        } else if let Some(language) = language(&path) {
            let relative = path.strip_prefix(root).into_diagnostic()?.to_path_buf();

            group.files.push(File { path: relative, language: Some(language.to_string()) });
        }
    }

    Ok(group)
}
