// Standard Uses
use std::collections::HashMap;

// Crate Uses

// External Uses
use miette::Result;
use knuffel::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct Switch {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(properties))]
    pub cases: HashMap<String, String>,

    #[knuffel(child, unwrap(argument))]
    pub index: String,

    #[knuffel(child, unwrap(argument))]
    pub default: Option<String>
}

#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct List {
    #[knuffel(argument)]
    pub name: String,

    #[knuffel(child, unwrap(arguments))]
    pub values: Vec<String>,

    #[knuffel(child, unwrap(argument))]
    pub index: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub default: Option<String>
}

