// Relative Modules
pub mod controller;
pub mod builder;
pub mod replacer;
pub mod maker;
pub mod resolver;
pub mod validation;

// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses
use crate::shaping::patch::builder::Builder;
use crate::shaping::patch::replacer::Replacer;
use crate::shaping::patch::maker::Maker;

// External Uses
use miette::{bail, IntoDiagnostic, Report, Result};
use crate::shaping::patch::resolver::{List, Switch};


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knus::Decode)]
pub struct Patch {
    #[knus(child, unwrap(argument))]
    pub enabled: bool,
    #[knus(child, unwrap(argument))]
    pub alias: String,
    #[knus(child, unwrap(argument))]
    pub file: String,
    #[knus(child)]
    pub actions: Actions,
}


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knus::Decode)]
pub struct Actions {
    #[knus(child, unwrap(children(name="builder")))]
    pub builders: Option<Vec<Builder>>,

    #[knus(child, unwrap(children(name="replacer")))]
    pub replacers: Option<Vec<Replacer>>,

    #[knus(child, unwrap(children(name="maker")))]
    pub makers: Option<Vec<Maker>>,

    #[knus(child, unwrap(children(name="switch")))]
    pub switchers: Option<Vec<Switch>>,

    #[knus(child, unwrap(children(name="list")))]
    pub lists: Option<Vec<List>>,
}

#[allow(unused)]
impl Patch {
    pub fn from_path(path: &Path) -> Result<Self> {
        let extension = path.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(path).into_diagnostic()?;
        let content = content.as_str();

        match extension {
            "kdl" => Self::from_kdl(path.file_name().unwrap().to_str().unwrap(), content),
            &_ => { bail!("Extension {} is not supported", extension) }
        }
    }

    pub fn from_kdl(filename: &str, content: &str) -> Result<Self> {
        knus::parse::<Patch>(filename, content).map_err(Report::new)
    }
}
