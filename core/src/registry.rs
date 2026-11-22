// Standard Uses
use std::collections::HashMap;
use std::path::Path;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::target::{Solution, TargetLoader};

// External Uses
use miette::{miette, Result};


#[derive(Default)]
pub struct Registry {
    backends: HashMap<String, Box<dyn LanguageBackend>>,
    extensions: HashMap<String, String>,
    loaders: HashMap<String, Box<dyn TargetLoader>>
}

impl Registry {
    pub fn add_backend(&mut self, backend: Box<dyn LanguageBackend>, extensions: &[&str]) {
        let name = backend.name().to_string();

        for extension in extensions {
            self.extensions.insert(extension.to_string(), name.clone());
        }

        self.backends.insert(name, backend);
    }

    pub fn add_loader(&mut self, loader: Box<dyn TargetLoader>) {
        self.loaders.insert(loader.name().to_string(), loader);
    }

    pub fn backend(&self, name: &str) -> Option<&dyn LanguageBackend> {
        self.backends.get(name).map(|backend| backend.as_ref())
    }

    pub fn backend_for(&self, path: &Path) -> Option<&dyn LanguageBackend> {
        let extension = path.extension()?.to_str()?;

        self.backend(self.extensions.get(extension)?)
    }

    pub fn loader(&self, name: &str) -> Option<&dyn TargetLoader> {
        self.loaders.get(name).map(|loader| loader.as_ref())
    }

    pub fn load_target(&self, name: &str, path: &Path) -> Result<Solution> {
        let loader = self.loader(name).ok_or_else(|| {
            let mut known: Vec<&str> = self.loaders.keys().map(String::as_str).collect();
            known.sort();

            miette!("The target type '{}' is not registered, known types are: {}", name, known.join(", "))
        })?;

        loader.load(path)
    }
}
