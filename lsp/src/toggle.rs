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
    pub text: String
}


pub fn toggle(outline: &Outline, text: &str) -> Option<Toggle> {
    let Some(node) = outline.top("enabled") else { return disable(outline, text) };
    let literal = node.arguments.first()?;
    let (title, text) = match literal.value.as_str() {
        "true" => ("Disable patch", "false"),
        "false" => ("Enable patch", "true"),
        _ => return None
    };

    Some(Toggle { title, node: node.name_range.start..literal.range.end, value: literal.range.clone(), text: text.to_string() })
}

fn disable(outline: &Outline, text: &str) -> Option<Toggle> {
    let node: &Node = outline.nodes.iter().find(|node| node.parent.is_none() && !node.commented)?;
    let start = node.name_range.start;
    let line = text[..start].rfind('\n').map(|line| line + 1).unwrap_or(0);
    let indentation = &text[line..start];

    let next = match (indentation.trim().is_empty(), text[start..].split('\n').next().is_some_and(|rest| rest.ends_with('\r'))) {
        (true, true) => format!("\r\n{indentation}"),
        (true, false) => format!("\n{indentation}"),
        (false, _) => "; ".to_string()
    };

    Some(Toggle { title: "Disable patch", node: node.name_range.clone(), value: start..start, text: format!("enabled false{next}") })
}
