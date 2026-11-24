// Standard Uses
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

// Crate Uses
use crate::document::{kind, DocumentKind};
use crate::syntax::{Node, Outline};

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration::{self, OperationConfiguration};
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::project::settings::{self, ProjectSettings};
use codeshaper::core::target::Options;


pub const TARGET: &str = "target";


#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub configuration: PathBuf,
    pub directory: PathBuf,
    pub own: Vec<PathBuf>,
    pub defaults: Options,
    pub overrides: Options
}

impl Target {
    pub fn options(&self) -> Options {
        let mut options = self.defaults.clone();
        options.extend(self.overrides.clone());

        options
    }

    pub fn name(&self) -> String {
        self.configuration.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
    }

    pub fn is_own(&self, file: &Path) -> bool {
        self.own.iter().any(|folder| file.starts_with(folder))
    }
}


pub fn project(path: &Path, text: &str) -> Option<PathBuf> {
    match kind(path) {
        DocumentKind::Patch { project } => Some(project),
        DocumentKind::Settings => path.parent().map(Path::to_path_buf),
        DocumentKind::Configuration => Some(path.parent()?.join(Outline::parse(text).argument("project")?)),
        DocumentKind::Other => None
    }
}

pub fn target_kind(path: &Path, text: &str) -> Option<String> {
    match kind(path) {
        DocumentKind::Settings => Outline::parse(text).argument(TARGET).map(str::to_string),
        DocumentKind::Configuration => saved_settings(path, text).map(|settings| settings.target.value),
        _ => None
    }
}

pub fn given(path: &Path, text: &str) -> Option<Options> {
    match kind(path) {
        DocumentKind::Settings => settings::from_kdl(text).ok().map(|settings| settings.target.options),
        DocumentKind::Configuration => configuration::from_kdl(text).ok().map(|configuration| configuration.target.options),
        _ => None
    }
}

pub fn targets(path: &Path, text: &str, given: Options) -> Vec<Target> {
    let Some(directory) = path.parent() else { return vec![] };

    match kind(path) {
        DocumentKind::Settings => {
            let Ok(configurations) = Operation::configurations_for(directory) else { return vec![] };

            configurations.into_iter()
                .filter_map(|configuration| {
                    let found = OperationConfiguration::from_file(&configuration).ok()?;
                    let directory = configuration.parent()?;

                    Some(Target {
                        directory: directory.join(&found.target.value),
                        own: found.own_folders(directory),
                        defaults: given.clone(),
                        overrides: found.target.options,
                        configuration
                    })
                })
                .filter(|target| target.directory.is_dir())
                .collect()
        }
        DocumentKind::Configuration => {
            let outline = Outline::parse(text);
            let Some(folder) = outline.argument(TARGET).map(|value| directory.join(value)).filter(|folder| folder.is_dir()) else { return vec![] };

            vec![Target {
                configuration: path.to_path_buf(),
                directory: folder,
                own: configuration::from_kdl(text).map(|configuration| configuration.own_folders(directory)).unwrap_or_default(),
                defaults: saved_settings(path, text).map(|settings| settings.target.options).unwrap_or_default(),
                overrides: given
            }]
        }
        _ => vec![]
    }
}

pub fn unknown(registry: &Registry, path: &Path, text: &str) -> Vec<(Range<usize>, &'static str, String)> {
    let outline = Outline::parse(text);
    let Some(name) = target_kind(path, text) else { return vec![] };

    let Some(loader) = registry.loader(&name) else {
        let message = format!("target: '{name}' is not a known target type, known types are: {}", registry.targets().join(", "));
        let argument = argument(&outline).filter(|_| kind(path) == DocumentKind::Settings);

        return argument.map(|range| (range, "target-not-found", message)).into_iter().collect()
    };

    let given = given(path, text).unwrap_or_default();
    let known = match loader.options() {
        [] => format!("'{name}' takes no options"),
        known => format!("the options of '{name}' are: {}", known.join(", "))
    };

    options(&outline)
        .filter(|node| given.contains_key(&node.name) && !loader.options().contains(&node.name.as_str()))
        .map(|node| (node.name_range.clone(), "option-not-found", format!("target: '{}' is not an option, {known}", node.name)))
        .collect()
}

pub fn argument(outline: &Outline) -> Option<Range<usize>> {
    Some(outline.top(TARGET)?.arguments.first()?.range.clone())
}

pub fn options(outline: &Outline) -> impl Iterator<Item = &Node> {
    outline.nodes.iter().filter(|node| node.parent.is_some_and(|parent| outline.is_top(parent, TARGET)))
}

fn saved_settings(path: &Path, text: &str) -> Option<ProjectSettings> {
    let content = fs::read_to_string(project(path, text)?.join("settings.kdl")).ok()?;

    settings::from_kdl(&content).ok()
}
