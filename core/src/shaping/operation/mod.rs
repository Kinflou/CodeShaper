// Relative Modules
pub mod configuration;
pub mod actions;
pub mod report;
pub mod session;

// Standard Uses
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::registry::Registry;
use crate::shaping::operation::configuration::{OperationConfiguration, ResultOptions};
use crate::shaping::operation::report::Report;
use crate::shaping::operation::session::Session;
use crate::shaping::patch::Patch;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::patch::validation::{validate_source, Diagnostic as PatchDiagnostic, DiagnosticKind, Severity};
use crate::shaping::project::Project;
use crate::target::File;

// External Uses
use glob::Pattern;
use miette::{bail, miette, Context, Diagnostic, IntoDiagnostic, Result};


pub struct Operation {
    pub configuration: OperationConfiguration,
    pub directory: PathBuf,
    pub project: Project,
}


impl Operation {
    pub fn from_file(path: &Path) -> Result<Self> {
        let configuration = OperationConfiguration::from_file(path)?;

        Self::from_configuration(configuration, path.parent().unwrap_or(Path::new("")))
    }

    pub fn from_configuration(configuration: OperationConfiguration, directory: &Path) -> Result<Self> {
        let project = Project::from_directory(&directory.join(&configuration.project))?;

        Ok(Self { configuration, directory: directory.to_path_buf(), project })
    }

    pub fn session<'a>(&'a self, registry: &'a Registry) -> Result<Session<'a>> {
        self.check_destinations()?;
        self.check_patches(registry)?;

        let target = self.directory.join(&self.configuration.target);
        let solution = registry.load_target(&self.project.settings.target, &target)?;

        Session::new(self, registry, solution)
    }

    pub fn run(&self, registry: &Registry) -> Result<Report> {
        self.session(registry)?.finish()
    }

    fn check_destinations(&self) -> Result<()> {
        match self.configuration.result {
            ResultOptions::BackupAndReplace if self.configuration.backup.is_empty() =>
                bail!("Operation '{}' backs up files but has no backup directory", self.configuration.name),
            ResultOptions::CreateNew if self.configuration.output.is_empty() =>
                bail!("Operation '{}' creates new files but has no output directory", self.configuration.name),
            _ => Ok(())
        }
    }

    pub fn check(&self, registry: &Registry) -> Result<Vec<(String, PatchDiagnostic)>> {
        let ast_set = &self.project.settings.ast_set;
        let mut found = vec![];

        for (name, patch) in &self.project.patches {
            let backend = patch_backend(registry, patch, ast_set);
            let diagnostics = validate_source(patch, &self.project.sources[name], backend);
            found.extend(diagnostics.into_iter().map(|diagnostic| (name.clone(), diagnostic)));
        }

        let target = self.directory.join(&self.configuration.target);
        let solution = registry.load_target(&self.project.settings.target, &target)?;
        let files = solution.files();

        for (name, patch) in &self.project.patches {
            if !patch.enabled { continue }

            let diagnostic = match Pattern::new(&patch.file) {
                Err(error) => PatchDiagnostic::new(
                    DiagnosticKind::IncorrectFilePattern, "patch", "file", error.to_string()
                ),
                Ok(pattern) if !files.iter().any(|file| pattern.matches_path(&file.path)) => PatchDiagnostic::new(
                    DiagnosticKind::FileNotMatched, "patch", "file",
                    format!("'{}' matches no file in the target", patch.file)
                ),
                Ok(_) => continue
            };
            let span = self.project.sources[name].spans.fields.get("file").copied();

            found.push((name.clone(), PatchDiagnostic { span, ..diagnostic }));
        }

        Ok(found)
    }

    fn check_patches(&self, registry: &Registry) -> Result<()> {
        let mut errors = vec![];

        for (name, patch) in &self.project.patches {
            if !patch.enabled { continue }

            let backend = patch_backend(registry, patch, &self.project.settings.ast_set);
            let source = &self.project.sources[name];

            for diagnostic in validate_source(patch, source, backend) {
                if diagnostic.severity != Severity::Error { continue }

                let title = format!(
                    "Patch {}: {} '{}': {}", name, diagnostic.action, diagnostic.field, diagnostic.message
                );
                errors.push(source.report(&diagnostic, title));
            }
        }

        if !errors.is_empty() {
            return Err(InvalidPatches { errors }.into());
        }

        Ok(())
    }

    fn matching_patches(&self, file: &File) -> Result<Vec<&String>> {
        let mut matching = vec![];

        for (name, patch) in &self.project.patches {
            if !patch.enabled { continue }

            let pattern = Pattern::new(&patch.file)
                .into_diagnostic()
                .wrap_err_with(|| format!("Patch {} has an invalid file pattern '{}'", name, patch.file))?;

            if pattern.matches_path(&file.path) {
                matching.push(name);
            }
        }

        Ok(matching)
    }

    fn apply_patch(&self, registry: &Registry, name: &str, file: &File, source: &str) -> Result<String> {
        let patch = &self.project.patches[name];

        self.backend(registry, patch, file)
            .and_then(|backend| PatchController::with(Rc::new(patch.clone())).apply(backend, source))
            .wrap_err_with(|| format!("Patch {} could not shape {}", name, file.path.display()))
    }

    fn backend<'r>(&self, registry: &'r Registry, patch: &Patch, file: &File) -> Result<&'r dyn LanguageBackend> {
        let named = [patch.alias.as_str(), self.project.settings.ast_set.as_str()].into_iter()
            .chain(file.language.as_deref())
            .find(|name| !name.is_empty());

        match named {
            Some(name) => registry.backend(name)
                .ok_or_else(|| miette!("No language backend named '{}' is registered", name)),
            None => registry.backend_for(&file.path)
                .ok_or_else(|| miette!("No language backend handles {}", file.path.display()))
        }
    }

    fn write(&self, directory: &Path, path: &Path, content: &str) -> Result<()> {
        let source = directory.join(path);

        match self.configuration.result {
            ResultOptions::Replace => write(&source, content),
            ResultOptions::BackupAndReplace => {
                let backup = self.directory.join(&self.configuration.backup).join(path);
                create_parent(&backup)?;
                fs::copy(&source, &backup)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Could not back up {}", source.display()))?;

                write(&source, content)
            }
            ResultOptions::CreateNew => write(&self.directory.join(&self.configuration.output).join(path), content)
        }
    }
}


pub fn patch_backend<'r>(registry: &'r Registry, patch: &Patch, ast_set: &str) -> Option<&'r dyn LanguageBackend> {
    match [patch.alias.as_str(), ast_set].into_iter().find(|name| !name.is_empty()) {
        Some(name) => registry.backend(name),
        None => registry.backend_for(Path::new(&patch.file))
    }
}

fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not create {}", parent.display()))?;
    }

    Ok(())
}

fn write(path: &Path, content: &str) -> Result<()> {
    create_parent(path)?;

    fs::write(path, content)
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not write {}", path.display()))
}


#[derive(Debug)]
struct RunFailed {
    errors: Vec<miette::Report>
}

impl Display for RunFailed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} patch application(s) failed, nothing was written", self.errors.len())
    }
}

impl std::error::Error for RunFailed {}

impl Diagnostic for RunFailed {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(self.errors.iter().map(|error| error.as_ref() as &dyn Diagnostic)))
    }
}


#[derive(Debug)]
struct InvalidPatches {
    errors: Vec<miette::Report>
}

impl Display for InvalidPatches {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} patch error(s) found, nothing was run", self.errors.len())
    }
}

impl std::error::Error for InvalidPatches {}

impl Diagnostic for InvalidPatches {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(self.errors.iter().map(|error| error.as_ref() as &dyn Diagnostic)))
    }
}
