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
    Unclosed { text: &'a str, span: Range<usize> },
}

impl ExpressionAST<'_> {
    pub fn errors(&self) -> Vec<ExpressionError> {
        match self {
            Self::Action(parts) | Self::ActionCall { arguments: parts, .. } => parts.iter().flat_map(Self::errors).collect(),
            Self::Unclosed { text, span } => {
                let expected = match text.ends_with('(') {
                    true => "arguments are captures, calls, numbers or text in quotes, with commas between them, followed by ')]'",
                    false => "expected a name followed by ']'"
                };

                vec![ExpressionError { message: format!("'{text}' is not a complete call: {expected}"), span: span.clone() }]
            }
            _ => vec![]
        }
    }
}

pub fn is_name(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '!')
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
