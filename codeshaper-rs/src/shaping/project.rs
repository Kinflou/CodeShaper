// Relative Modules
pub mod settings;

// Standard Uses
use std::path::{Path, PathBuf};

// Crate Uses
use crate::target;
use crate::shaping::patch::Patch;
use crate::shaping::project::settings::ProjectSettings;
use crate::target::file_map::FileMap;

// External Uses
use anyhow::Result;
use anyhow::bail;
use glob::glob;


// #[derive(Default, Clone)]
// #[derive(Default)]
pub struct Project {
    pub directory: String,
    pub settings: ProjectSettings,
    pub patches: Vec<Patch>,
    pub file_map: FileMap,
}

impl Project {

    pub fn from_directory(path: &Path) -> Result<Self> {
        if !path.is_dir() {
            bail!("Expected directory, got {} instead", path.display());
        }

        if !path.exists() {
            bail!("Project settings directory does not exist {:?}", path.to_str().unwrap());
        }

        let settings = ProjectSettings::from_path(path)?;
        
        Ok(Self {
            directory: path.to_str().unwrap().to_string(),
            settings,
            patches: vec![],
            file_map: FileMap::new()
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let settings = ProjectSettings::from_path(path).unwrap();

        let target = target::from_type(settings.target.as_str(), path)?;

        Ok(Self {
            settings,
            directory: path.to_str().unwrap().to_string(),
            patches: vec![],
            file_map: FileMap::from_target(target),
        })
    }

    pub fn load_patches(&self) {
        let mut patches: Vec<Patch> = vec![];
        let search = format!("{}/patches/**/*.*", &self.directory);

        let mut paths: Vec<PathBuf> = vec![];

        let search = glob(&search).expect("Failed to read glob pattern");

        for entry in search{
            match entry {
                Ok(path) => paths.push(path),
                Err(e) => println!("{:?}", e),
            }
        }

        for path in paths {
            let patch = Patch::from_path(path.as_path());

            patches.push(patch.unwrap());
        }

    }
}

