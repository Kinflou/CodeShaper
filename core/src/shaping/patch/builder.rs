// Standard USes

// Crate Uses
use crate::shaping::patch::Actions;

// External Uses
use eyre::Result;
use serde::{Serialize, Deserialize};


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knuffel::Decode)]
#[derive(Serialize, Deserialize)]
pub struct Builder {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub location: String,
    #[serde(default)]
    #[knuffel(child, unwrap(argument))]
    pub reference_location: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub r#match: String,
    #[knuffel(child, unwrap(argument))]
    pub build: String,
    // #[knuffel(child, unwrap(argument))]
    pub actions: Actions,
}

