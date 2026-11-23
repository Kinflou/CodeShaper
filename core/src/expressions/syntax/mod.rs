// Relative Modules
pub mod parser;

// Standard Uses
use std::fmt::{Display, Formatter};
use std::ops::Range;

// Crate Uses

// External Uses
use miette::Diagnostic;


#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ExpressionAST<'a> {
    Action(Vec<ExpressionAST<'a>>),
    Text(&'a str),
    ActionCall { name: &'a str, arguments: Vec<ExpressionAST<'a>>, span: Range<usize> },
    LocalCall { name: &'a str, span: Range<usize> },
    ArgumentCall { name: &'a str, span: Range<usize> },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionError {
    pub message: String,
    pub span: Range<usize>
}

impl Display for ExpressionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExpressionError {}

impl Diagnostic for ExpressionError {}
