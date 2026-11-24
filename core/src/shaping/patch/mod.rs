// Relative Modules
pub mod controller;
pub mod file;
pub mod builder;
pub mod replacer;
pub mod maker;
pub mod resolver;
pub mod validation;
pub mod source;
pub mod trace;
pub mod pattern;

// Standard Uses
use std::{fs, io};
use std::borrow::Cow;
use std::path::Path;

// Crate Uses
use crate::shaping::lines::unix;
use crate::shaping::patch::builder::Builder;
use crate::shaping::patch::file::{NamedPatch, PatchFile};
use crate::shaping::patch::replacer::Replacer;
use crate::shaping::patch::maker::Maker;
use crate::shaping::patch::source::PatchSource;

// External Uses
use glob::Pattern;
use miette::{bail, miette, IntoDiagnostic, Report, Result};
use crate::shaping::patch::resolver::{List, Switch};


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knus::Decode)]
pub struct Patch {
    #[knus(child, unwrap(argument))]
    pub enabled: bool,
    #[knus(child, unwrap(argument), default)]
    pub alias: String,
    #[knus(child, unwrap(argument))]
    pub file: String,
    #[knus(child)]
    pub actions: Actions,
}


#[derive(Default, Clone, Debug, PartialEq)]
#[derive(knus::Decode)]
pub struct Actions {
    #[knus(child, unwrap(children(name="builder")))]
    pub builders: Option<Vec<Builder>>,

    #[knus(child, unwrap(children(name="replacer")))]
    pub replacers: Option<Vec<Replacer>>,

    #[knus(child, unwrap(children(name="maker")))]
    pub makers: Option<Vec<Maker>>,

    #[knus(child, unwrap(children(name="switch")))]
    pub switchers: Option<Vec<Switch>>,

    #[knus(child, unwrap(children(name="list")))]
    pub lists: Option<Vec<List>>,
}

#[allow(unused)]
impl Patch {
    pub fn from_path(path: &Path) -> Result<Self> {
        let extension = path.extension().unwrap().to_str().unwrap();
        let content = fs::read_to_string(path).into_diagnostic()?;
        let content = content.as_str();

        match extension {
            "kdl" => Self::from_kdl(path.file_name().unwrap().to_str().unwrap(), content),
            &_ => { bail!("Extension {} is not supported", extension) }
        }
    }

    pub fn from_kdl(filename: &str, content: &str) -> Result<Self> {
        let patches = Self::decode_all(filename, content)?;

        Ok(only(filename, patches, |(name, _)| name.as_deref())?.1)
    }

    pub fn load(path: &Path) -> Result<(Self, PatchSource)> {
        Self::load_with(path, &|path| fs::read_to_string(path))
    }

    pub fn load_with(path: &Path, read: &dyn Fn(&Path) -> io::Result<String>) -> Result<(Self, PatchSource)> {
        let (name, content) = read_patch(path, read)?;

        Self::parse(&name, &content)
    }

    pub fn load_all(path: &Path) -> Result<Vec<NamedPatch>> {
        Self::load_all_with(path, &|path| fs::read_to_string(path))
    }

    pub fn load_all_with(path: &Path, read: &dyn Fn(&Path) -> io::Result<String>) -> Result<Vec<NamedPatch>> {
        let (name, content) = read_patch(path, read)?;

        Self::parse_all(&name, &content)
    }

    pub fn parse(name: &str, content: &str) -> Result<(Self, PatchSource)> {
        let NamedPatch { patch, source, .. } = only(name, Self::parse_all(name, content)?, |named| named.name.as_deref())?;

        Ok((patch, source))
    }

    pub fn parse_all(name: &str, content: &str) -> Result<Vec<NamedPatch>> {
        let patches = Self::decode_all(name, content)?;
        let sources = PatchSource::parse_all(name, content)?;

        Ok(patches.into_iter().zip(sources).map(|((name, patch), source)| NamedPatch { name, patch, source }).collect())
    }

    fn decode_all(name: &str, content: &str) -> Result<Vec<(Option<String>, Self)>> {
        let file = knus::parse::<PatchFile>(name, content).map_err(Report::new)?;

        match unix(content) {
            Cow::Borrowed(_) => Ok(file.patches),
            Cow::Owned(content) => Ok(knus::parse::<PatchFile>(name, &content).map_err(Report::new)?.patches)
        }
    }

    pub fn matches(&self, file: &Path) -> bool {
        Pattern::new(&self.file).is_ok_and(|pattern| pattern.matches_path(file))
    }
}


fn read_patch(path: &Path, read: &dyn Fn(&Path) -> io::Result<String>) -> Result<(String, String)> {
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default();
    if extension != "kdl" {
        bail!("Extension {} is not supported", extension)
    }

    let content = read(path).into_diagnostic()?;
    let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();

    Ok((name, content))
}

fn only<T>(file: &str, mut patches: Vec<T>, name: impl Fn(&T) -> Option<&str>) -> Result<T> {
    if patches.len() > 1 {
        let names: Vec<&str> = patches.iter().filter_map(name).collect();
        bail!("Expected one patch, {} has several: {}", file, names.join(", "))
    }

    patches.pop().ok_or_else(|| miette!("{} has no patch", file))
}
