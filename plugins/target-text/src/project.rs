// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses

// External Uses
use miette::{Context, IntoDiagnostic, Report, Result};


#[derive(knus::Decode)]
pub struct Group {
    #[knus(argument)]
    pub name: String,
    #[knus(child, unwrap(arguments), default)]
    pub files: Vec<String>,
    #[knus(child, unwrap(arguments), default)]
    pub include: Vec<String>,
    #[knus(child, unwrap(arguments), default)]
    pub exclude: Vec<String>,
    #[knus(children(name="group"))]
    pub groups: Vec<Group>
}

#[derive(knus::Decode)]
pub struct Project {
    #[knus(child, unwrap(argument))]
    pub name: String,
    #[knus(children(name="group"))]
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
    knus::parse::<Project>("project.kdl", content).map_err(Report::new)
}
