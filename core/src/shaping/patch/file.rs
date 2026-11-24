// Standard Uses
use std::collections::HashSet;

// Crate Uses
use crate::shaping::patch::Patch;
use crate::shaping::patch::source::PatchSource;
use crate::shaping::target::text;

// External Uses
use knus::ast::SpannedNode;
use knus::decode::Context;
use knus::errors::DecodeError;
use knus::traits::{DecodeChildren, ErrorSpan};


pub const BLOCK: &str = "patch";


#[derive(Debug, Clone)]
pub struct NamedPatch {
    pub name: Option<String>,
    pub patch: Patch,
    pub source: PatchSource
}

pub(crate) struct PatchFile {
    pub patches: Vec<(Option<String>, Patch)>
}

impl<S: ErrorSpan> DecodeChildren<S> for PatchFile {
    fn decode_children(nodes: &[SpannedNode<S>], ctx: &mut Context<S>) -> Result<Self, DecodeError<S>> {
        if !nodes.iter().any(is_block) {
            return Ok(Self { patches: vec![(None, Patch::decode_children(nodes, ctx)?)] })
        }

        let mut names = HashSet::new();
        let mut patches = vec![];

        for node in nodes {
            if !is_block(node) {
                let message = format!("'{}' is outside of the file's patches", &**node.node_name);
                ctx.emit_error(DecodeError::unexpected(&node.node_name, "node", message));
                continue
            }

            let named = name(node, ctx).and_then(|name| match names.insert(name.clone()) {
                true => Ok(name),
                false => Err(DecodeError::unexpected(
                    &node.arguments[0].literal, "name", format!("the file already has a patch named '{name}'")
                ))
            });

            match (named, body(node, ctx)) {
                (Ok(name), Ok(patch)) => patches.push((Some(name), patch)),
                (named, patch) => [named.err(), patch.err()].into_iter().flatten().for_each(|error| ctx.emit_error(error))
            }
        }

        Ok(Self { patches })
    }
}


pub fn is_block<S>(node: &SpannedNode<S>) -> bool {
    &**node.node_name == BLOCK
}

fn name<S: ErrorSpan>(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<String, DecodeError<S>> {
    if let Some(name) = node.properties.keys().next() {
        ctx.emit_error(DecodeError::unexpected(name, "property", "a patch takes its name only"));
    }
    if let Some(extra) = node.arguments.get(1) {
        ctx.emit_error(DecodeError::unexpected(&extra.literal, "argument", "a patch takes its name only"));
    }
    let Some(argument) = node.arguments.first() else {
        return Err(DecodeError::missing(node, "the patch has no name"))
    };

    let name = text(argument)?;
    if name.trim().is_empty() {
        return Err(DecodeError::unexpected(&argument.literal, "name", "the patch's name is empty"))
    }

    Ok(name.to_string())
}

fn body<S: ErrorSpan>(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<Patch, DecodeError<S>> {
    let children = node.children.as_ref().map(|children| children.as_slice()).unwrap_or_default();

    Patch::decode_children(children, ctx).map_err(|error| match error {
        DecodeError::MissingNode { message } => DecodeError::missing(node, message),
        error => error
    })
}
