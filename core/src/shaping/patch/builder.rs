// Standard USes

// Crate Uses
use crate::shaping::patch::Actions;

// External Uses
use miette::Result;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knuffel::Decode)]
pub struct Builder {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub location: String,
    #[knuffel(child, unwrap(argument))]
    pub reference_location: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub r#match: String,
    #[knuffel(child, unwrap(argument))]
    pub build: String,
    #[knuffel(child, unwrap(argument), default)]
    pub collect: bool,
    #[knuffel(child, unwrap(argument), default)]
    pub separator: String,
    #[knuffel(child, default)]
    pub actions: Actions,
}

