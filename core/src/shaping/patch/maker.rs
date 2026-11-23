// Standard Uses

// Crate Uses

// External Uses
use knus::Decode;


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(Decode)]
pub struct Maker {
    #[knus(argument)]
    pub name: String,
    #[knus(child, unwrap(argument))]
    pub prepare: Option<String>,
    #[knus(child, unwrap(argument), default)]
    pub flags: String,
    #[knus(child, unwrap(argument))]
    pub make: String,
}

