// Standard Uses

// Crate Uses
use crate::registry::Registry;
use crate::shaping::target::text;
use crate::target::File;

// External Uses
use glob::Pattern;
use knus::ast::SpannedNode;
use knus::decode::Context;
use knus::errors::DecodeError;
use knus::traits::ErrorSpan;
use knus::Decode;
use miette::{bail, Result};


pub const TARGET: &str = "target";


#[derive(Default, Debug, Clone, PartialEq)]
pub enum AstSet {
    Language(String),
    #[default]
    Target,
    Files(Vec<(String, Vec<Pattern>)>)
}

impl AstSet {
    pub fn fixed(&self) -> Option<&str> {
        match self {
            Self::Language(name) => Some(name),
            _ => None
        }
    }

    pub fn language<'a>(&'a self, file: &'a File) -> Option<&'a str> {
        match self {
            Self::Language(name) => Some(name),
            Self::Target => file.language.as_deref(),
            Self::Files(lines) => lines.iter().rev()
                .find(|(_, patterns)| patterns.iter().any(|pattern| pattern.matches_path(&file.path)))
                .map(|(name, _)| name.as_str())
        }
    }

    pub fn names(&self) -> Vec<&str> {
        match self {
            Self::Language(name) => vec![name],
            Self::Target => vec![],
            Self::Files(lines) => lines.iter().map(|(name, _)| name.as_str()).collect()
        }
    }

    pub fn check(&self, registry: &Registry) -> Result<()> {
        if let Some(name) = self.names().into_iter().find(|name| registry.backend(name).is_none()) {
            bail!("'ast-set' names '{}', which is not a known language, known languages are: {}", name, registry.languages().join(", "))
        }

        Ok(())
    }
}

impl<S: ErrorSpan> Decode<S> for AstSet {
    fn decode_node(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<Self, DecodeError<S>> {
        if let Some(name) = node.properties.keys().next() {
            ctx.emit_error(DecodeError::unexpected(name, "property", "'ast-set' takes no properties"));
        }
        if let Some(extra) = node.arguments.get(1) {
            ctx.emit_error(DecodeError::unexpected(&extra.literal, "argument", "'ast-set' takes one language, or a block mapping languages to files"));
        }

        match (node.arguments.first(), &node.children) {
            (Some(value), Some(_)) => Err(DecodeError::unexpected(
                &value.literal, "argument", "'ast-set' takes either a language or a block, not both"
            )),
            (Some(value), None) => match text(value)? {
                "" => Err(DecodeError::unexpected(&value.literal, "argument", "'ast-set' is empty, name a language or use \"target\"")),
                TARGET => Ok(Self::Target),
                name => Ok(Self::Language(name.to_string()))
            },
            (None, Some(children)) if !children.is_empty() => {
                let lines = children.iter().map(line).collect::<Result<Vec<_>, _>>()?;

                Ok(Self::Files(lines))
            }
            _ => Err(DecodeError::missing(node, "'ast-set' names no language"))
        }
    }
}

fn line<S: ErrorSpan>(node: &SpannedNode<S>) -> Result<(String, Vec<Pattern>), DecodeError<S>> {
    if let Some(name) = node.properties.keys().next() {
        return Err(DecodeError::unexpected(name, "property", "a language takes file patterns only"))
    }
    if node.children.is_some() {
        return Err(DecodeError::unexpected(&node.node_name, "node", "a language takes file patterns on its line, not a block"))
    }
    if node.arguments.is_empty() {
        return Err(DecodeError::missing(node, format!("'{}' has no file patterns", &**node.node_name)))
    }

    let patterns = node.arguments.iter()
        .map(|value| Pattern::new(text(value)?).map_err(|error| DecodeError::conversion(&value.literal, error)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((node.node_name.to_string(), patterns))
}
