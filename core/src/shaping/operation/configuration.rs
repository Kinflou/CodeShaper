// Standard Uses
use std::fs;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::shaping::target::Target;

// External Uses
use dunce::canonicalize;
use miette::{bail, Context, IntoDiagnostic, Report, Result};


#[derive(Debug, Clone, PartialEq)]
#[derive(knus::Decode)]
pub struct OperationConfiguration {
    #[knus(child, unwrap(argument))]
    pub name: String,
    #[knus(child, unwrap(argument))]
    pub project: String,
    #[knus(child)]
    pub target: Target,
    #[knus(child, unwrap(argument))]
    pub output: String,

    #[knus(child, unwrap(argument), default)]
    pub backup: String,

    #[knus(child, unwrap(argument))]
    pub result: ResultOptions
}

#[derive(Debug, Clone, PartialEq)]
#[derive(knus::DecodeScalar)]
pub enum ResultOptions {
    Replace,
    BackupAndReplace,
    CreateNew
}

impl OperationConfiguration {

    pub fn from_file(path: &Path) -> Result<OperationConfiguration> {
        let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default();
        let content = fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not load operation configuration {}", path.display()))?;

        from_extension(content.as_str(), extension)
    }

    pub fn own_folders(&self, directory: &Path) -> Vec<PathBuf> {
        let Ok(target) = canonicalize(directory.join(&self.target.value)) else { return vec![] };

        [&self.project, &self.output, &self.backup].into_iter()
            .filter(|folder| !folder.is_empty())
            .filter_map(|folder| canonicalize(directory.join(folder)).ok()?.strip_prefix(&target).ok().map(Path::to_path_buf))
            .filter(|folder| !folder.as_os_str().is_empty())
            .collect()
    }

    pub fn get_real_project_directory(&self) -> Result<Metadata> {
        /*
        let directory = format!("{}/{}", &self.project, &self.project);

        if fs::metadata(&directory).unwrap().is_dir() {
            return Ok(fs::metadata(&directory).unwrap());
        }
        */

        if fs::metadata(&self.project).unwrap().is_dir() {
            return Ok(fs::metadata(&self.project).unwrap());
        }

        bail!("Could not determine directory")
    }

}


fn from_extension(content: &str, extension: &str) -> Result<OperationConfiguration> {
    match extension {
        "kdl" => from_kdl(content),
        &_ => { bail!("Operation configuration extension '{}' is not supported", extension) }
    }
}

pub fn from_kdl(content: &str) -> Result<OperationConfiguration> {
    knus::parse::<OperationConfiguration>("configuration.kdl", content)
        .map_err(Report::new)
}

