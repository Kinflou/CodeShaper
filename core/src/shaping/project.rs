// Relative Modules
pub mod settings;

// Standard Uses
use std::collections::BTreeMap;
use std::path::Path;

// Crate Uses
use crate::shaping::patch::Patch;
use crate::shaping::project::settings::ProjectSettings;

// External Uses
use miette::{bail, Context, IntoDiagnostic, Result};
use glob::glob;


pub struct Project {
    pub directory: String,
    pub settings: ProjectSettings,
    pub patches: BTreeMap<String, Patch>,
}

impl Project {
    pub fn from_directory(path: &Path) -> Result<Self> {
        if !path.is_dir() {
            bail!("Expected a shaping project directory, got {} instead", path.display());
        }

        let settings = ProjectSettings::from_path(path)?;
        let patches = load_patches(&path.join("patches"))?;

        Ok(Self {
            directory: path.to_str().unwrap().to_string(),
            settings,
            patches,
        })
    }
}


pub fn load_patches(directory: &Path) -> Result<BTreeMap<String, Patch>> {
    let mut patches = BTreeMap::new();
    let search = directory.join("**").join("*.*");

    let entries = glob(&search.to_string_lossy())
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not search for patches in {}", directory.display()))?;

    for entry in entries {
        let path = entry.into_diagnostic()?;
        let name = path.strip_prefix(directory).into_diagnostic()?.to_string_lossy().to_string();

        let patch = Patch::from_path(&path)
            .wrap_err_with(|| format!("Could not load patch {}", path.display()))?;

        patches.insert(name, patch);
    }

    Ok(patches)
}
