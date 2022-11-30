// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use anyhow::{anyhow, bail, Context, Result};
use glob::glob;
use serde::Serialize;
use serde::Deserialize;
use knuffel::parse;


#[derive(Default, Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct ProjectSettings {
    pub name: String,
    pub target: String,
    pub ast_set: String,
    pub description: String
}

impl ProjectSettings {

    pub fn from_path(path: &Path) -> Result<Self> {
        let mut search = glob(format!("{}settings.*", path.to_str().unwrap()).as_str())
            .context("Failed to search for glob pattern")
            .unwrap();

        let settings = search.next()
            .context(format!("Could not find any file named 'settings.*' as '*' being a supported extension\
            \nat {}", path.to_str().unwrap()))
            .unwrap()?;

        let extension = settings.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(&settings)
            .context("Could not read file")
            .unwrap();

        from_extension(content.as_str(), extension)
    }
}


fn from_extension(content: &str, extension: &str) -> Result<ProjectSettings> {
    match extension {
        "json" => from_json(content),
        "json5" => from_json5(content),
        "kdl" => from_kdl(content),
        &_ => { bail!("Project settings extension '{}' is not supported", extension) }
    }
}

fn from_json(content: &str) -> Result<ProjectSettings> {
    serde_json::from_str(content).map_err(|err| anyhow!("{}", err))
}

fn from_json5(content: &str) -> Result<ProjectSettings> {
    json5::from_str(content).map_err(|err| anyhow!("{}", err))
}


fn from_kdl(content: &str) -> Result<ProjectSettings> {
    parse::<ProjectSettings>("settings.kdl", content)
        .map_err(|err| anyhow!("{:#?}", err))
}

