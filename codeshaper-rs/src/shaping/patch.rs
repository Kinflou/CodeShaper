// Relative Modules
pub mod controller;

// Standard Uses
use std::fs;
use std::path::Path;
use std::collections::HashMap;


// Crate Uses

// External Uses
use serde::Serialize;
use serde::Deserialize;
use knuffel::Decode;
use anyhow::{anyhow, bail, Result};
use knuffel::parse;


#[derive(Default, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Patch {
    pub enable: bool,
    pub name: String,
    pub alias: String,
    pub file: String,
    pub actions: Actions,
}


#[derive(Default, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Actions {
    pub builders: Vec<Builder>,
    pub replacers: Vec<Replacer>,
    pub makers: Vec<Maker>,
    pub resolvers: Vec<Resolver>,
}

#[derive(Default, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Builder {
    pub location: String,
    pub build: String,
    pub reference_location: String,
    pub r#match: String,
    pub actions: Actions
}


#[derive(Default, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Replacer {
    pub location: String,
    pub from: String,
    pub to: String,
    pub flags: String,
    pub reference_location: String,
    pub reference: String,
    pub actions: Actions
}


#[derive(Default, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Maker {
    pub prepare: String,
    pub make: String,

}

#[derive(Default, Clone, Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Resolver {
    pub mode: String,
    pub cases: HashMap<String, String>,
    pub list: Vec<String>,
    pub index: String,
    pub default: String
}


impl Patch {
    pub fn from_path(path: &Path) -> Result<Self> {
        let extension = path.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(path)?;

        from_extension(content.as_str(), extension)
    }
}


fn from_extension(content: &str, extension: &str) -> Result<Patch> {
    match extension {
        "json5" => from_json5(content),
        "kdl" => from_kdl(content),
        &_ => { bail!("Extension {} is not supported", extension) }
    }
}

fn from_json5(content: &str) -> Result<Patch> {
    json5::from_str(content).map_err(|err| anyhow!("{}", err))
}

fn from_kdl(content: &str) -> Result<Patch> {
    parse::<Patch>("settings.kdl", content)
        .map_err(|err| anyhow!("{:#?}", err))
}
