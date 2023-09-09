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
    pub location: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub flags: String,
    #[serde(default)]
    pub reference_location: String,
    pub reference: String,
    pub actions: Actions
}

