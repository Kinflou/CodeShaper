// Standard Uses

// Crate Uses

// External Uses
use pest::Parser;
use pest_derive::Parser;


#[derive(Parser)]
#[grammar = "src/shaping/operation/expressions/expression.pest"]
pub struct ExpressionParser;

