// Relative Modules
pub mod ast_set;
pub mod settings;

// Standard Uses
use std::collections::BTreeMap;
use std::{fs, io};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

// Crate Uses
use crate::shaping::patch::Patch;
use crate::shaping::patch::source::PatchSource;
use crate::shaping::project::settings::ProjectSettings;

// External Uses
use miette::{bail, Context, IntoDiagnostic, Result};
use glob::glob;


pub struct Project {
    pub directory: String,
    pub settings: ProjectSettings,
    pub patches: BTreeMap<String, Patch>,
    pub sources: BTreeMap<String, PatchSource>,
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
        let (patches, sources) = load_patches(&path.join("patches"), read)?;

        Ok(Self {
            directory: path.to_str().unwrap().to_string(),
            settings,
            patches,
            sources,
        })
    }
}


pub fn load_patches(
    directory: &Path, read: &dyn Fn(&Path) -> io::Result<String>
) -> Result<(BTreeMap<String, Patch>, BTreeMap<String, PatchSource>)> {
    let mut patches = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let search = directory.join("**").join("*.*");

    let entries = glob(&search.to_string_lossy())
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not search for patches in {}", directory.display()))?;

    for entry in entries {
        let path = entry.into_diagnostic()?;
        let name = path.strip_prefix(directory).into_diagnostic()?.to_string_lossy().replace(MAIN_SEPARATOR, "/");

        let (patch, source) = Patch::load_with(&path, read)
            .wrap_err_with(|| format!("Could not load patch {}", path.display()))?;

        patches.insert(name.clone(), patch);
        sources.insert(name, source);
    }

    Ok((patches, sources))
}
