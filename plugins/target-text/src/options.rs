// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use knus::ast::{Literal, SpannedNode};
use knus::span::Span;
use miette::{Report, Result};


pub const FILES: &str = "files";
pub const EXCLUDE: &str = "exclude";
pub const OPTIONS: [&str; 2] = [FILES, EXCLUDE];


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Files,
    Exclude
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub field: Field,
    pub value: String,
    pub range: Range<usize>
}

impl Field {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            FILES => Some(Self::Files),
            EXCLUDE => Some(Self::Exclude),
            _ => None
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Files => FILES,
            Self::Exclude => EXCLUDE
        }
    }
}


pub fn listed(content: &str) -> Result<Vec<Listed>> {
    let document = knus::parse_ast::<Span>("target", content).map_err(Report::new)?;
    let mut found = vec![];

    for child in document.nodes.iter().filter(|node| &**node.node_name == "target").flat_map(children) {
        let Some(field) = Field::named(&child.node_name) else { continue };
        let mut add = |value: &str, span: &Span| found.push(Listed { field, value: value.to_string(), range: span.0..span.1 });

        for argument in &child.arguments {
            if let Literal::String(value) = &*argument.literal { add(value, argument.literal.span()) }
        }
        for entry in children(child) {
            add(&entry.node_name, entry.node_name.span());
        }
    }

    Ok(found)
}

fn children(node: &SpannedNode<Span>) -> impl Iterator<Item = &SpannedNode<Span>> {
    node.children.iter().flat_map(|children| children.iter())
}
