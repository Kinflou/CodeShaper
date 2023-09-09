// Standard Uses

// Crate Uses

// External Uses


#[derive(Debug, PartialEq)]
pub enum Error {
    LocationNotFound,
    ActionNotFound,
    IncorrectArgs,
    IncorrectRegex,
}
