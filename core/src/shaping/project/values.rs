// Standard Uses
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{Display, Formatter};

// Crate Uses
use crate::expressions::syntax;
use crate::shaping::lines::unix;
use crate::shaping::project::settings::{ProjectSettings, FILE};
use crate::shaping::target::text;

// External Uses
use knus::ast::SpannedNode;
use knus::decode::Context;
use knus::errors::DecodeError;
use knus::traits::ErrorSpan;
use knus::Decode;
use miette::{bail, Diagnostic, Result};


const DEFAULT: &str = "default";


#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    pub name: String,
    pub value: String
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub default: Option<String>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Constant,
    Set,
    Configuration,
    Default,
    Missing
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub name: String,
    pub value: Option<String>,
    pub source: Source
}

impl Declared {
    pub fn kind(&self) -> &'static str {
        match self.source {
            Source::Constant => "constant",
            _ => "variable"
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Values {
    texts: HashMap<String, String>
}

impl Values {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.texts.get(name).map(String::as_str)
    }
}

impl From<&[Declared]> for Values {
    fn from(declared: &[Declared]) -> Self {
        let texts = declared.iter()
            .filter_map(|declared| Some((declared.name.clone(), unix(declared.value.as_deref()?).into_owned())))
            .collect();

        Self { texts }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotSet {
    pub names: Vec<String>
}

impl Display for NotSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let names: Vec<String> = self.names.iter().map(|name| format!("'{name}'")).collect();
        let (variables, have, them) = match names.len() {
            1 => ("Variable", "has", "it"),
            _ => ("Variables", "have", "them")
        };

        write!(
            f, "{variables} {} {have} no value, set {them} in the configuration or with --set, or give {them} a default in {FILE}",
            names.join(", ")
        )
    }
}

impl std::error::Error for NotSet {}

impl Diagnostic for NotSet {}


pub fn declared(settings: &ProjectSettings, configured: &BTreeMap<String, String>, set: &BTreeMap<String, String>) -> Vec<Declared> {
    let constants = settings.constants.iter()
        .map(|Constant { name, value }| Declared { name: name.clone(), value: Some(value.clone()), source: Source::Constant });

    let variables = settings.variables.iter().map(|Variable { name, default }| {
        let sources = [(set.get(name), Source::Set), (configured.get(name), Source::Configuration), (default.as_ref(), Source::Default)];
        let found = sources.into_iter().find_map(|(value, source)| Some((value?.clone(), source)));

        match found {
            Some((value, source)) => Declared { name: name.clone(), value: Some(value), source },
            None => Declared { name: name.clone(), value: None, source: Source::Missing }
        }
    });

    constants.chain(variables).collect()
}

pub fn check_settable(settings: &ProjectSettings, name: &str) -> Result<()> {
    if settings.variables.iter().any(|variable| variable.name == name) { return Ok(()) }

    if settings.constants.iter().any(|constant| constant.name == name) {
        bail!("'{name}' is a constant, constants can't be set")
    }

    bail!("{FILE} declares no variable named '{name}'")
}

pub fn check_set(declared: &[Declared], called: &BTreeSet<String>) -> Result<()> {
    let names: Vec<String> = declared.iter()
        .filter(|declared| declared.value.is_none() && called.contains(&declared.name))
        .map(|declared| declared.name.clone())
        .collect();

    if !names.is_empty() { return Err(NotSet { names }.into()) }

    Ok(())
}

pub fn is_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(syntax::is_name)
}


#[derive(Default, Clone)]
struct Names {
    known: HashSet<String>
}

impl<S: ErrorSpan> Decode<S> for Constant {
    fn decode_node(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<Self, DecodeError<S>> {
        plain(node, 2, "a constant takes its name and its value", ctx);
        if let Some(property) = node.properties.keys().next() {
            ctx.emit_error(DecodeError::unexpected(property, "property", "a constant takes its name and its value"));
        }

        let name = name(node, ctx)?;
        let value = node.arguments.get(1).ok_or_else(|| DecodeError::missing(node, format!("constant '{name}' has no value")))?;

        Ok(Self { name, value: text(value)?.to_string() })
    }
}

impl<S: ErrorSpan> Decode<S> for Variable {
    fn decode_node(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<Self, DecodeError<S>> {
        plain(node, 1, "a variable takes its name, its value goes in default=\"…\"", ctx);
        if let Some(property) = node.properties.keys().find(|property| &****property != DEFAULT) {
            ctx.emit_error(DecodeError::unexpected(property, "property", "a variable takes its name and default=\"…\""));
        }

        let name = name(node, ctx)?;
        let default = node.properties.get(DEFAULT).map(text).transpose()?.map(str::to_string);

        Ok(Self { name, default })
    }
}

fn plain<S: ErrorSpan>(node: &SpannedNode<S>, arguments: usize, message: &str, ctx: &mut Context<S>) {
    if let Some(extra) = node.arguments.get(arguments) {
        ctx.emit_error(DecodeError::unexpected(&extra.literal, "argument", message));
    }
    if node.children.is_some() {
        ctx.emit_error(DecodeError::unexpected(&node.node_name, "node", message));
    }
}

fn name<S: ErrorSpan>(node: &SpannedNode<S>, ctx: &mut Context<S>) -> Result<String, DecodeError<S>> {
    let kind = &**node.node_name;
    let argument = node.arguments.first().ok_or_else(|| DecodeError::missing(node, format!("the {kind} has no name")))?;
    let name = text(argument)?;

    if !is_name(name) {
        return Err(DecodeError::unexpected(
            &argument.literal, "name", format!("'{name}' can't be called, names have letters, digits, '_', '-' and '!'")
        ))
    }

    let mut names = ctx.get::<Names>().cloned().unwrap_or_default();
    if !names.known.insert(name.to_string()) {
        return Err(DecodeError::unexpected(&argument.literal, "name", format!("the project already declares '{name}'")))
    }
    ctx.set(names);

    Ok(name.to_string())
}
