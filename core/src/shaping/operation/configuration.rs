// Standard Uses
use std::fs;
use std::fs::Metadata;
use std::path::Path;

// Crate Uses

// External Uses
use miette::{bail, Context, IntoDiagnostic, Report, Result};


#[derive(Debug, Clone, PartialEq)]
#[derive(knus::Decode)]
pub struct OperationConfiguration {
    #[knus(child, unwrap(argument))]
    pub name: String,
    #[knus(child, unwrap(argument))]
    pub project: String,
    #[knus(child, unwrap(argument))]
    pub target: String,
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

