// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use miette::{miette, bail, Context, IntoDiagnostic, Result};
use glob::glob;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct Group {
    #[knuffel(argument)]
    pub name: String,
    #[serde(default)]
    #[knuffel(child, unwrap(arguments), default)]
    pub files: Vec<String>,
    #[serde(default)]
    #[knuffel(children(name="group"))]
    pub groups: Vec<Group>
}

#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct Project {
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(children(name="group"))]
    pub groups: Vec<Group>,
}

impl Project {
    pub fn from_path(path: &Path) -> Result<Self> {
        let mut search = glob(format!("{}project.*", path.to_str().unwrap()).as_str())
            .into_diagnostic()
            .context("Failed to search for glob pattern")?;

        let project = search.next()
            .ok_or_else(|| miette!("Could not find any file named 'project.*' where '*' \
             being a supported extension at {}", path.to_str().unwrap()))?
            .into_diagnostic()?;

        let extension = project.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(&project)
            .into_diagnostic()
            .context("Could not read file")?;

        from_extension(content.as_str(), extension)
    }
}


fn from_extension(content: &str, extension: &str) -> Result<Project> {
    match extension {
        "json5" => from_json5(content),
        "kdl" => from_kdl(content),
        &_ => { bail!("Cannot parse given format extension {}", extension) }
    }
}

pub fn from_json5(content: &str) -> Result<Project> {
    json5::from_str(content).map_err(|err| miette!("{err}"))
}

pub fn from_kdl(content: &str) -> Result<Project> {
    knuffel::parse::<Project>("project.kdl", content).map_err(miette::Report::new)
}
