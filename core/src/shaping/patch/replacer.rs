// Standard Uses

// Crate Uses
use crate::shaping::patch::Actions;

// External Uses
use knus::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct Replacer {
    #[knus(child, unwrap(argument))]
    pub location: String,
    #[knus(child, unwrap(argument))]
    pub from: String,
    #[knus(child, unwrap(argument))]
    pub to: String,
    #[knus(child, unwrap(argument), default)]
    pub flags: String,
    #[knus(child, unwrap(argument), default)]
    pub reference_location: String,
    #[knus(child, unwrap(argument), default)]
    pub reference: String,
    #[knus(child, default)]
    pub actions: Actions
}

impl Replacer {
    pub fn target_location(&self) -> &str {
        if self.reference_location.is_empty() { &self.location } else { &self.reference_location }
    }
}
