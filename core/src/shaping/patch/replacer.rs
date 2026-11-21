// Standard Uses

// Crate Uses
use crate::shaping::patch::Actions;

// External Uses
use serde::{Serialize, Deserialize};
use knuffel::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(Decode)]
pub struct Replacer {
    #[knuffel(child, unwrap(argument))]
    pub location: String,
    #[knuffel(child, unwrap(argument))]
    pub from: String,
    #[knuffel(child, unwrap(argument))]
    pub to: String,
    #[serde(default)]
    #[knuffel(child, unwrap(argument), default)]
    pub flags: String,
    #[serde(default)]
    #[knuffel(child, unwrap(argument), default)]
    pub reference_location: String,
    #[knuffel(child, unwrap(argument), default)]
    pub reference: String,
    #[knuffel(child, default)]
    pub actions: Actions
}

impl Replacer {
    pub fn target_location(&self) -> &str {
        if self.reference_location.is_empty() { &self.location } else { &self.reference_location }
    }
}
