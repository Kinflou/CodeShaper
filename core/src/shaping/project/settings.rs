// Standard Uses
use std::borrow::Cow;
use std::fs;
use std::path::Path;

// Crate Uses
use crate::shaping::lines::unix;
use crate::shaping::project::ast_set::AstSet;
use crate::shaping::project::values::{Constant, Variable};
use crate::shaping::target::Target;

// External Uses
use miette::{bail, Context, IntoDiagnostic, Report, Result};
use knus::parse;


pub const FILE: &str = "project.kdl";


#[derive(Default, Debug, Clone, PartialEq)]
#[derive(knus::Decode)]
pub struct ProjectSettings {
    #[knus(child, unwrap(argument))]
    pub name: String,
    #[knus(child)]
    pub target: Target,
    #[knus(child)]
    pub ast_set: AstSet,
    #[knus(child, unwrap(argument))]
    pub description: String,
    #[knus(children(name="constant"))]
    pub constants: Vec<Constant>,
    #[knus(children(name="variable"))]
    pub variables: Vec<Variable>
}

impl ProjectSettings {

    pub fn from_path(path: &Path) -> Result<Self> {
        let settings = path.join(FILE);

        if !settings.is_file() {
            bail!("Could not find {FILE} at {}", path.display());
        }

        let content = fs::read_to_string(&settings)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not read {}", settings.display()))?;

        from_kdl(&content)
    }
}


pub fn from_kdl(content: &str) -> Result<ProjectSettings> {
    let settings = parse::<ProjectSettings>(FILE, content).map_err(Report::new)?;

    match unix(content) {
        Cow::Borrowed(_) => Ok(settings),
        Cow::Owned(content) => parse::<ProjectSettings>(FILE, &content).map_err(Report::new)
    }
}

