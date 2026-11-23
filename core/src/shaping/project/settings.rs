// Standard Uses
use std::borrow::Cow;
use std::fs;
use std::path::Path;

// Crate Uses
use crate::shaping::lines::unix;
use crate::shaping::project::ast_set::AstSet;

// External Uses
use miette::{miette, bail, Context, IntoDiagnostic, Report, Result};
use glob::glob;
use knus::parse;


#[derive(Default, Debug, Clone, PartialEq)]
#[derive(knus::Decode)]
pub struct ProjectSettings {
    #[knus(child, unwrap(argument))]
    pub name: String,
    #[knus(child, unwrap(argument))]
    pub target: String,
    #[knus(child)]
    pub ast_set: AstSet,
    #[knus(child, unwrap(argument))]
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
        "kdl" => from_kdl(content),
        &_ => { bail!("Project settings extension '{}' is not supported", extension) }
    }
}



pub fn from_kdl(content: &str) -> Result<ProjectSettings> {
    let settings = parse::<ProjectSettings>("settings.kdl", content).map_err(Report::new)?;

    match unix(content) {
        Cow::Borrowed(_) => Ok(settings),
        Cow::Owned(content) => parse::<ProjectSettings>("settings.kdl", &content).map_err(Report::new)
    }
}

