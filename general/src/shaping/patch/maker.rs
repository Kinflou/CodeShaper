// Standard Uses

// Crate Uses

// External Uses
use serde::{Serialize, Deserialize};
use knuffel::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Maker {
    #[knuffel(argument)]
    pub name: String,
    #[serde(default)]
    #[knuffel(child, unwrap(argument))]
    pub prepare: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub make: String,
}

