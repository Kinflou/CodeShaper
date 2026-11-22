// Standard USes

// Crate Uses
use crate::shaping::patch::Actions;

// External Uses


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knus::Decode)]
pub struct Builder {
    #[knus(argument)]
    pub name: String,
    #[knus(child, unwrap(argument))]
    pub location: String,
    #[knus(child, unwrap(argument))]
    pub reference_location: Option<String>,
    #[knus(child, unwrap(argument))]
    pub r#match: String,
    #[knus(child, unwrap(argument))]
    pub build: String,
    #[knus(child, unwrap(argument), default)]
    pub collect: bool,
    #[knus(child, unwrap(argument), default)]
    pub separator: String,
    #[knus(child, default)]
    pub actions: Actions,
}

