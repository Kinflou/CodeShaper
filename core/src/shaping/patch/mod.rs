// Relative Modules
pub mod controller;
pub mod builder;
pub mod replacer;
pub mod maker;
pub mod resolver;

// Standard Uses
use std::fs;
use std::path::Path;

// Crate Uses
use crate::shaping::patch::builder::Builder;
use crate::shaping::patch::replacer::Replacer;
use crate::shaping::patch::maker::Maker;
use crate::shaping::patch::resolver::Resolver;

// External Uses
use eyre::{anyhow, bail, Result};
use serde::{Serialize, Deserialize};


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct Patch {
    #[knuffel(child, unwrap(argument))]
    pub enabled: bool,
    #[knuffel(child, unwrap(argument))]
    pub alias: String,
    #[knuffel(child, unwrap(argument))]
    pub file: String,
    #[knuffel(child)]
    pub actions: Actions,
}


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(knuffel::Decode)]
pub struct Actions {
    #[knuffel(child, unwrap(children(name="builder")))]
    pub builders: Option<Vec<Builder>>,
    #[knuffel(child, unwrap(children(name="replacer")))]
    pub replacers: Option<Vec<Replacer>>,
    #[knuffel(child, unwrap(children(name="maker")))]
    pub makers: Option<Vec<Maker>>,
    #[knuffel(child, unwrap(children(name="resolver")))]
    pub resolvers: Option<Vec<Resolver>>,
}

#[allow(unused)]
impl Patch {
    pub fn from_path(path: &Path) -> Result<Self> {
        let extension = path.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(path)?;
        let content = content.as_str();

        match extension {
            "json5" => from_json5(content),
            "kdl" => from_kdl(path.file_name().unwrap().to_str().unwrap(), content),
            &_ => { bail!("Extension {} is not supported", extension) }
        }
    }
}

fn from_json5(content: &str) -> Result<Patch> {
    json5::from_str(content).map_err(|err| anyhow!("{}", err))
}

fn from_kdl(filename: &str, content: &str) -> Result<Patch> {
    match knuffel::parse::<Patch>(filename, content) {
        Ok(patch) => Ok(patch),
        Err(err) => Err(anyhow!("{:?}", miette::Report::new(err)))
    }
}
