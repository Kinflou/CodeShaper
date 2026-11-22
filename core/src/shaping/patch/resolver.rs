// Standard Uses
use std::collections::HashMap;

// Crate Uses

// External Uses
use knus::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct Switch {
    #[knus(argument)]
    pub name: String,

    #[knus(child, unwrap(properties))]
    pub cases: HashMap<String, String>,

    #[knus(child, unwrap(argument))]
    pub index: String,

    #[knus(child, unwrap(argument))]
    pub default: Option<String>
}

#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct List {
    #[knus(argument)]
    pub name: String,

    #[knus(child, unwrap(arguments))]
    pub values: Vec<String>,

    #[knus(child, unwrap(argument))]
    pub index: Option<String>,

    #[knus(child, unwrap(argument))]
    pub default: Option<String>
}

