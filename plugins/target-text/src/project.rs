// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use miette::{Context, IntoDiagnostic, Result};


#[derive(knuffel::Decode)]
pub struct Group {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(child, unwrap(arguments), default)]
    pub files: Vec<String>,
    #[knuffel(child, unwrap(arguments), default)]
    pub include: Vec<String>,
    #[knuffel(child, unwrap(arguments), default)]
    pub exclude: Vec<String>,
    #[knuffel(children(name="group"))]
    pub groups: Vec<Group>
}

#[derive(knuffel::Decode)]
pub struct Project {
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(children(name="group"))]
    pub groups: Vec<Group>,
}

impl Project {
    pub fn from_path(path: &Path) -> Result<Self> {
        let project = path.join("project.kdl");
        let content = fs::read_to_string(&project)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not read {}", project.display()))?;

        from_kdl(&content)
    }
}


pub fn from_kdl(content: &str) -> Result<Project> {
    knuffel::parse::<Project>("project.kdl", content).map_err(miette::Report::new)
}
