// Standard Uses
use std::collections::HashMap;

// Crate Uses

// External Uses
use anyhow::Result;
use serde::{Serialize, Deserialize};
use knuffel::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Resolver {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub mode: String,
    //#[knuffel(child, unwrap(argument))]
    pub cases: HashMap<String, String>,
    //#[knuffel(child, unwrap(argument))]
    pub list: Vec<String>,
    #[knuffel(child, unwrap(argument))]
    pub index: String,
    #[knuffel(child, unwrap(argument))]
    pub default: String
}

