// Standard Uses
use std::ops::Range;

// Crate Uses
use crate::syntax::{Node, Outline};

// External Uses


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toggle {
    pub title: &'static str,
    pub node: Range<usize>,
    pub value: Range<usize>,
    pub text: &'static str
}


pub fn toggle(outline: &Outline) -> Option<Toggle> {
    let node: &Node = outline.top("enabled")?;
    let literal = node.arguments.first()?;
    let (title, text) = match literal.value.as_str() {
        "true" => ("Disable patch", "false"),
        "false" => ("Enable patch", "true"),
        _ => return None
    };

    Some(Toggle { title, node: node.name_range.start..literal.range.end, value: literal.range.clone(), text })
}
