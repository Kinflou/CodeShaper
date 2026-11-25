// Relative Modules
pub mod ast_set;
pub mod settings;
pub mod values;

// Standard Uses
use std::collections::HashMap;
use std::{fs, io};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

// Crate Uses
use crate::shaping::patch::Patch;
use crate::shaping::patch::file::NamedPatch;
use crate::shaping::patch::source::PatchSource;
use crate::shaping::project::settings::ProjectSettings;

// External Uses
use miette::{bail, Context, IntoDiagnostic, Result};
use glob::glob;


pub struct Project {
    pub directory: String,
    pub settings: ProjectSettings,
    pub patches: Vec<ProjectPatch>,
}

#[derive(Debug, Clone)]
pub struct ProjectPatch {
    pub name: String,
    pub path: PathBuf,
    pub patch: Patch,
    pub source: PatchSource
}

impl Project {
    pub fn containing(patch: &Path) -> Option<PathBuf> {
        patch.ancestors().skip(1)
            .filter(|ancestor| ancestor.file_name().is_some_and(|name| name == "patches"))
            .filter_map(Path::parent)
            .find(|project| project.join(settings::FILE).is_file())
            .map(Path::to_path_buf)
    }

    pub fn from_directory(path: &Path) -> Result<Self> {
        Self::from_directory_with(path, &|path| fs::read_to_string(path))
    }

    pub fn from_directory_with(path: &Path, read: &dyn Fn(&Path) -> io::Result<String>) -> Result<Self> {
        if !path.is_dir() {
            bail!("Expected a shaping project directory, got {} instead", path.display());
        }

        let settings = ProjectSettings::from_path(path)?;
        let patches = load_patches(&path.join("patches"), read)?;

        Ok(Self {
            directory: path.to_str().unwrap().to_string(),
            settings,
            patches,
        })
    }

    pub fn patch(&self, name: &str) -> Option<&ProjectPatch> {
        self.patches.iter().find(|patch| patch.name == name)
    }
}


pub fn patch_files(directory: &Path) -> Result<Vec<(String, PathBuf)>> {
    let search = directory.join("**").join("*.*");

    let entries = glob(&search.to_string_lossy())
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not search for patches in {}", directory.display()))?;

    let mut files = entries
        .map(|entry| {
            let path = entry.into_diagnostic()?;
            let name = path.strip_prefix(directory).into_diagnostic()?.to_string_lossy().replace(MAIN_SEPARATOR, "/");

            Ok((name, path))
        })
        .collect::<Result<Vec<_>>>()?;
    files.sort();

    Ok(files)
}

pub fn load_patches(directory: &Path, read: &dyn Fn(&Path) -> io::Result<String>) -> Result<Vec<ProjectPatch>> {
    let mut patches = vec![];
    let mut files: HashMap<String, String> = HashMap::new();

    for (file, path) in patch_files(directory)? {
        let found = Patch::load_all_with(&path, read)
            .wrap_err_with(|| format!("Could not load patch {}", path.display()))?;

        for NamedPatch { name, patch, source } in found {
            let name = name.unwrap_or_else(|| file.clone());
            if let Some(other) = files.insert(name.clone(), file.clone()) {
                bail!("Patch '{name}' is in both {other} and {file}")
            }

            patches.push(ProjectPatch { name, path: path.clone(), patch, source });
        }
    }

    Ok(patches)
}
