// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use miette::{miette, bail, Context, IntoDiagnostic, Result};
use glob::glob;
use serde::Serialize;
use serde::Deserialize;
use knuffel::parse;


#[derive(Default, Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct ProjectSettings {
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub target: String,
    #[knuffel(child, unwrap(argument))]
    pub ast_set: String,
    #[knuffel(child, unwrap(argument))]
    pub description: String
}

impl ProjectSettings {

    pub fn from_path(path: &Path) -> Result<Self> {
        let mut search = glob(format!("{}settings.*", path.to_str().unwrap()).as_str())
            .into_diagnostic()
            .context("Failed to search for glob pattern")
            .unwrap();

        let settings = search.next()
            .ok_or_else(|| miette!("Could not find any file named 'settings.*' as '*' being a supported extension\
            \nat {}", path.to_str().unwrap()))?
            .into_diagnostic()?;

        let extension = settings.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(&settings)
            .into_diagnostic()
            .context("Could not read file")
            .unwrap();

        from_extension(content.as_str(), extension)
    }
}


fn from_extension(content: &str, extension: &str) -> Result<ProjectSettings> {
    match extension {
        "json5" => from_json5(content),
        "kdl" => from_kdl(content),
        &_ => { bail!("Project settings extension '{}' is not supported", extension) }
    }
}


fn from_json5(content: &str) -> Result<ProjectSettings> {
    json5::from_str(content).map_err(|err| miette!("{}", err))
}


fn from_kdl(content: &str) -> Result<ProjectSettings> {
    parse::<ProjectSettings>("settings.kdl", content)
        .map_err(|error| miette::miette!("{error}"))
}

