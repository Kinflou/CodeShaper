// Standard Uses

// Crate Uses
use crate::target::Options;

// External Uses
use knus::ast::{Literal, SpannedNode, Value};
use knus::decode::Context;
use knus::errors::DecodeError;
use knus::traits::ErrorSpan;
use knus::Decode;


#[derive(Default, Debug, Clone, PartialEq)]
pub struct Target {
    pub value: String,
    pub options: Options
}

impl From<&str> for Target {
    fn from(value: &str) -> Self {
        Self { value: value.to_string(), options: Options::new() }
    }
}

impl<S: ErrorSpan> Decode<S> for Target {
    fn decode_node(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<Self, DecodeError<S>> {
        if let Some(name) = node.properties.keys().next() {
            ctx.emit_error(DecodeError::unexpected(name, "property", "'target' takes no properties, its options go in a block"));
        }
        if let Some(extra) = node.arguments.get(1) {
            ctx.emit_error(DecodeError::unexpected(&extra.literal, "argument", "'target' takes one value, its options go in a block"));
        }

        let value = node.arguments.first().ok_or_else(|| DecodeError::missing(node, "'target' has no value"))?;
        let mut options = Options::new();

        for child in node.children.iter().flat_map(|children| children.iter()) {
            if options.insert(child.node_name.to_string(), entries(child)?).is_some() {
                return Err(DecodeError::unexpected(&child.node_name, "node", format!("'{}' is given twice", &**child.node_name)))
            }
        }

        Ok(Self { value: text(value)?.to_string(), options })
    }
}

fn entries<S: ErrorSpan>(node: &SpannedNode<S>) -> Result<Vec<String>, DecodeError<S>> {
    if let Some(name) = node.properties.keys().next() {
        return Err(DecodeError::unexpected(name, "property", format!("'{}' takes no properties", &**node.node_name)))
    }

    let mut found = node.arguments.iter()
        .map(|value| text(value).map(str::to_string))
        .collect::<Result<Vec<_>, _>>()?;

    for entry in node.children.iter().flat_map(|children| children.iter()) {
        if let Some(extra) = entry.arguments.first() {
            return Err(DecodeError::unexpected(&extra.literal, "argument", "a block takes one entry per line"))
        }
        if let Some(name) = entry.properties.keys().next() {
            return Err(DecodeError::unexpected(name, "property", "a block takes one entry per line"))
        }
        if entry.children.is_some() {
            return Err(DecodeError::unexpected(&entry.node_name, "node", "an entry takes no block"))
        }

        found.push(entry.node_name.to_string());
    }

    Ok(found)
}

pub(crate) fn text<S: ErrorSpan>(value: &Value<S>) -> Result<&str, DecodeError<S>> {
    match &*value.literal {
        Literal::String(text) => Ok(text),
        _ => Err(DecodeError::unexpected(&value.literal, "argument", "expected text in quotes"))
    }
}
