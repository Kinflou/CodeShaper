// Standard Uses
use std::fs;
use std::fs::Metadata;
use std::path::Path;

// Crate Uses

// External Uses
use miette::{bail, Context, IntoDiagnostic, Result};
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct OperationConfiguration {
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub project: String,
    #[knuffel(child, unwrap(argument))]
    pub target: String,
    #[knuffel(child, unwrap(argument))]
    pub output: String,

    #[serde(default)]
    #[knuffel(child, unwrap(argument), default)]
    pub backup: String,

    #[knuffel(child, unwrap(argument))]
    pub result: ResultOptions
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(knuffel::DecodeScalar)]
pub enum ResultOptions {
    Replace,
    BackupAndReplace,
    CreateNew
}

impl OperationConfiguration {

    pub fn from_file(path: &Path) -> Result<OperationConfiguration> {
        let extension = path.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(path)
            .into_diagnostic()
            .context("Could not load operation configuration")
            .unwrap();

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
        "json5" => from_json5(content),
        "kdl" => from_kdl(content),
        &_ => { bail!("") }
    }
}

pub fn from_json5(content: &str) -> Result<OperationConfiguration> {
    json5::from_str(content).into_diagnostic().context("Could not load JSON5 operation settings")
}

pub fn from_kdl(content: &str) -> Result<OperationConfiguration> {
    knuffel::parse::<OperationConfiguration>("configuration.kdl", content)
        .map_err(miette::Report::new)
}

