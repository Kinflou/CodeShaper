// Standard Uses
use std::collections::HashMap;
use std::path::Path;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::target::{Options, Solution, TargetLoader};

// External Uses
use miette::{bail, miette, Result};


#[derive(Default)]
pub struct Registry {
    backends: HashMap<String, Box<dyn LanguageBackend>>,
    loaders: HashMap<String, Box<dyn TargetLoader>>
}

impl Registry {
    pub fn add_backend(&mut self, backend: Box<dyn LanguageBackend>) {
        self.backends.insert(backend.name().to_string(), backend);
    }

    pub fn add_loader(&mut self, loader: Box<dyn TargetLoader>) {
        self.loaders.insert(loader.name().to_string(), loader);
    }

    pub fn backend(&self, name: &str) -> Option<&dyn LanguageBackend> {
        self.backends.get(name).map(|backend| backend.as_ref())
    }

    pub fn languages(&self) -> Vec<&str> {
        let mut found: Vec<&str> = self.backends.keys().map(String::as_str).collect();
        found.sort();

        found
    }

    pub fn loader(&self, name: &str) -> Option<&dyn TargetLoader> {
        self.loaders.get(name).map(|loader| loader.as_ref())
    }

    pub fn targets(&self) -> Vec<&str> {
        let mut found: Vec<&str> = self.loaders.keys().map(String::as_str).collect();
        found.sort();

        found
    }

    pub fn load_target(&self, name: &str, path: &Path, options: &Options) -> Result<Solution> {
        let loader = self.loader(name).ok_or_else(|| {
            miette!("The target type '{}' is not registered, known types are: {}", name, self.targets().join(", "))
        })?;

        if let Some(option) = options.keys().find(|option| !loader.options().contains(&option.as_str())) {
            match loader.options() {
                [] => bail!("The target type '{}' takes no options, but has '{}'", name, option),
                known => bail!("The target type '{}' has no option '{}', its options are: {}", name, option, known.join(", "))
            }
        }

        loader.load(path, options)
    }
}
