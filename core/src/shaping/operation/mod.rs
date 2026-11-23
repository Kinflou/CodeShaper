// Relative Modules
pub mod configuration;
pub mod actions;
pub mod report;
pub mod session;

// Standard Uses
use std::fmt::{Display, Formatter};
use std::{fs, io};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::registry::Registry;
use crate::shaping::operation::configuration::{OperationConfiguration, ResultOptions};
use crate::shaping::operation::report::Report;
use crate::shaping::operation::session::{Change, Session};
use crate::shaping::patch::Patch;
use crate::shaping::patch::controller::PatchController;
use crate::shaping::patch::source::PatchSource;
use crate::shaping::patch::trace::Trace;
use crate::shaping::patch::validation::{validate, validate_source, Diagnostic as PatchDiagnostic, DiagnosticKind, Severity};
use crate::shaping::project::Project;
use crate::shaping::project::ast_set::AstSet;
use crate::target::{File, Solution};

// External Uses
use dunce::canonicalize;
use glob::Pattern;
use miette::{bail, miette, Context, Diagnostic, IntoDiagnostic, Result};


pub struct Operation {
    pub configuration: OperationConfiguration,
    pub directory: PathBuf,
    pub project: Project,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LocationMatches {
    pub files: usize,
    pub matched_files: usize,
    pub places: usize
}

#[derive(Debug, Clone, PartialEq)]
pub struct Places {
    pub path: PathBuf,
    pub source: String,
    pub ranges: Vec<Range<usize>>
}


impl Operation {
    pub fn from_file(path: &Path) -> Result<Self> {
        let configuration = OperationConfiguration::from_file(path)?;

        Self::from_configuration(configuration, path.parent().unwrap_or(Path::new("")))
    }

    pub fn for_project(project: &Path) -> Result<Self> {
        Self::from_file(&Self::configuration_for(project)?)
    }

    pub fn configuration_for(project: &Path) -> Result<PathBuf> {
        let project = canonicalize(project).into_diagnostic()?;
        let parent = project.parent().ok_or_else(|| miette!("{} has no parent directory", project.display()))?;

        let mut candidates: Vec<PathBuf> = fs::read_dir(parent).into_diagnostic()?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension == "kdl"))
            .collect();
        candidates.sort();

        candidates.into_iter()
            .find(|path| {
                let Some(found) = fs::read_to_string(path).ok().and_then(|content| configuration::from_kdl(&content).ok()) else {
                    return false
                };
                canonicalize(parent.join(found.project)).is_ok_and(|directory| directory == project)
            })
            .ok_or_else(|| miette!("No operation configuration next to {} points at it", project.display()))
    }

    pub fn from_configuration(configuration: OperationConfiguration, directory: &Path) -> Result<Self> {
        Self::from_configuration_with(configuration, directory, &|path| fs::read_to_string(path))
    }

    pub fn from_configuration_with(
        configuration: OperationConfiguration, directory: &Path, read: &dyn Fn(&Path) -> io::Result<String>
    ) -> Result<Self> {
        let project = Project::from_directory_with(&directory.join(&configuration.project), read)?;

        Ok(Self { configuration, directory: directory.to_path_buf(), project })
    }

    pub fn session<'a>(&'a self, registry: &'a Registry) -> Result<Session<'a>> {
        self.check_destinations()?;
        let solution = self.target(registry)?;
        self.check_patches(registry, &solution.files())?;

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
        self.project.settings.ast_set.check(registry)?;
        let solution = self.target(registry)?;
        let files = solution.files();
        let mut found = vec![];

        for (name, patch) in &self.project.patches {
            let source = &self.project.sources[name];
            let (backends, diagnostics) = self.languages(registry, patch, &files);

            found.extend(validate_source(patch, source, backends).into_iter().map(|diagnostic| (name.clone(), diagnostic)));
            found.extend(diagnostics.into_iter().map(|diagnostic| (name.clone(), spanned(diagnostic, source))));
        }

        for (name, patch) in &self.project.patches {
            let diagnostic = file_diagnostic(patch, &self.project.sources[name], &files);
            found.extend(diagnostic.map(|diagnostic| (name.clone(), diagnostic)));
        }

        Ok(found)
    }

    fn check_patches(&self, registry: &Registry, files: &[&File]) -> Result<()> {
        self.project.settings.ast_set.check(registry)?;
        let mut errors = vec![];

        for (name, patch) in &self.project.patches {
            if !patch.enabled { continue }

            let source = &self.project.sources[name];
            let (backends, diagnostics) = self.languages(registry, patch, files);
            let diagnostics = diagnostics.into_iter().map(|diagnostic| spanned(diagnostic, source));

            for diagnostic in validate_source(patch, source, backends).into_iter().chain(diagnostics) {
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

    fn check_patch(&self, registry: &Registry, name: &str, patch: &Patch, files: &[File]) -> Result<()> {
        self.project.settings.ast_set.check(registry)?;
        let (backends, diagnostics) = self.languages(registry, patch, &files.iter().collect::<Vec<_>>());
        let errors: Vec<String> = validate(patch, backends).into_iter().chain(diagnostics)
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .map(|diagnostic| format!("{} '{}': {}", diagnostic.action, diagnostic.field, diagnostic.message))
            .collect();

        if !errors.is_empty() {
            bail!("Patch {} has errors: {}", name, errors.join("; "))
        }

        Ok(())
    }

    pub fn target(&self, registry: &Registry) -> Result<Solution> {
        registry.load_target(&self.project.settings.target, &self.directory.join(&self.configuration.target))
    }

    pub fn languages<'r>(
        &self, registry: &'r Registry, patch: &Patch, files: &[&File]
    ) -> (Vec<&'r dyn LanguageBackend>, Vec<PatchDiagnostic>) {
        let ast_set = &self.project.settings.ast_set;
        let unknown = |field, message| PatchDiagnostic::new(DiagnosticKind::LanguageNotFound, "patch", field, message);

        if !patch.alias.is_empty() {
            return match registry.backend(&patch.alias) {
                Some(backend) => (vec![backend], vec![]),
                None => (vec![], vec![unknown("alias", format!("'{}' is not a known language, known languages are: {}", patch.alias, registry.languages().join(", ")))])
            }
        }
        if let Some(name) = ast_set.fixed() {
            return (registry.backend(name).into_iter().collect(), vec![])
        }

        let Ok(pattern) = Pattern::new(&patch.file) else { return (vec![], vec![]) };
        let mut names: Vec<&str> = vec![];
        let mut without: Vec<&File> = vec![];

        for file in files.iter().filter(|file| pattern.matches_path(&file.path)) {
            match ast_set.language(file) {
                Some(name) if !names.contains(&name) => names.push(name),
                Some(_) => {}
                None => without.push(file)
            }
        }

        let mut diagnostics = vec![];
        if let Some(file) = without.first() {
            let others = match without.len() - 1 { 0 => String::new(), others => format!(" and {others} more file(s)") };
            let reason = if *ast_set == AstSet::Target { "the target gives none" } else { "no line of 'ast-set' matches" };
            diagnostics.push(unknown("file", format!("'{}'{others} has no language, {reason}", file.path.display())));
        }
        for name in names.iter().filter(|name| registry.backend(name).is_none()) {
            diagnostics.push(unknown("file", format!("matches files in '{name}', which is not a known language")));
        }

        (names.into_iter().filter_map(|name| registry.backend(name)).collect(), diagnostics)
    }

    pub fn preview(&self, registry: &Registry, name: &str, patch: &Patch) -> Result<Vec<Change>> {
        let (directory, files) = self.matching_files(registry, name, patch)?;
        self.check_patch(registry, name, patch, &files)?;

        let changes = files.iter()
            .map(|file| {
                let path = directory.join(&file.path);
                let before = fs::read_to_string(&path)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Could not read {}", path.display()));

                match before {
                    Ok(before) => {
                        let result = self.apply_patch(registry, name, patch, file, &before);
                        Change { file: file.path.clone(), patch: name.to_string(), before, result }
                    }
                    Err(error) => Change { file: file.path.clone(), patch: name.to_string(), before: String::new(), result: Err(error) }
                }
            })
            .collect();

        Ok(changes)
    }

    pub fn locate(&self, registry: &Registry, name: &str, patch: &Patch, location: &str) -> Result<LocationMatches> {
        let places = self.places(registry, name, patch, location)?;

        Ok(LocationMatches {
            files: places.len(),
            matched_files: places.iter().filter(|places| !places.ranges.is_empty()).count(),
            places: places.iter().map(|places| places.ranges.len()).sum()
        })
    }

    pub fn places(&self, registry: &Registry, name: &str, patch: &Patch, location: &str) -> Result<Vec<Places>> {
        let (directory, files) = self.matching_files(registry, name, patch)?;

        let places = files.iter()
            .map(|file| {
                let path = directory.join(&file.path);
                let source = fs::read_to_string(&path).unwrap_or_default();
                let ranges = self.backend(registry, patch, file)
                    .and_then(|backend| backend.locate(&source, location))
                    .unwrap_or_default();

                Places { path, source, ranges }
            })
            .collect();

        Ok(places)
    }

    pub fn check_file(&self, registry: &Registry, patch: &Patch, source: &PatchSource) -> Result<Vec<PatchDiagnostic>> {
        let solution = self.target(registry)?;
        let files = solution.files();
        let (_, diagnostics) = self.languages(registry, patch, &files);

        Ok(diagnostics.into_iter()
            .map(|diagnostic| spanned(diagnostic, source))
            .chain(file_diagnostic(patch, source, &files))
            .collect())
    }

    pub fn trace(&self, registry: &Registry, name: &str, patch: &Patch, limit: usize) -> Result<Trace> {
        self.trace_while(registry, name, patch, limit, &|| true)
    }

    pub fn trace_while(
        &self, registry: &Registry, name: &str, patch: &Patch, limit: usize, keep_going: &dyn Fn() -> bool
    ) -> Result<Trace> {
        let (directory, files) = self.matching_files(registry, name, patch)?;
        self.check_patch(registry, name, patch, &files)?;
        let patch = Rc::new(patch.clone());
        let mut trace = Trace::new(limit);

        for file in &files {
            if !keep_going() { bail!("Tracing patch {} was stopped", name) }

            let Ok(source) = fs::read_to_string(directory.join(&file.path)) else { continue };
            let Ok(backend) = self.backend(registry, &patch, file) else { continue };

            let mut controller = PatchController::traced(patch.clone(), limit);
            if controller.apply(backend, &source).is_err() { continue }

            if let Some(traced) = controller.trace.take() {
                trace.merge(traced.into_inner(), &file.path);
            }
        }

        Ok(trace)
    }

    pub fn matching_files(&self, registry: &Registry, name: &str, patch: &Patch) -> Result<(PathBuf, Vec<File>)> {
        let pattern = file_pattern(name, patch)?;
        let solution = self.target(registry)?;

        let files = solution.files().into_iter()
            .filter(|file| pattern.matches_path(&file.path))
            .cloned()
            .collect();

        Ok((solution.directory, files))
    }

    fn matching_patches(&self, file: &File) -> Result<Vec<&String>> {
        let mut matching = vec![];

        for (name, patch) in &self.project.patches {
            if !patch.enabled { continue }

            if file_pattern(name, patch)?.matches_path(&file.path) {
                matching.push(name);
            }
        }

        Ok(matching)
    }

    fn apply_patch(&self, registry: &Registry, name: &str, patch: &Patch, file: &File, source: &str) -> Result<String> {
        self.backend(registry, patch, file)
            .and_then(|backend| PatchController::with(Rc::new(patch.clone())).apply(backend, source))
            .wrap_err_with(|| format!("Patch {} could not shape {}", name, file.path.display()))
    }

    pub fn backend<'r>(&self, registry: &'r Registry, patch: &Patch, file: &File) -> Result<&'r dyn LanguageBackend> {
        let name = match patch.alias.as_str() {
            "" => self.project.settings.ast_set.language(file),
            alias => Some(alias)
        }.ok_or_else(|| miette!("{} has no language", file.path.display()))?;

        registry.backend(name).ok_or_else(|| miette!("No language backend named '{}' is registered", name))
    }

    fn back_up(&self, directory: &Path, path: &Path) -> Result<()> {
        if self.configuration.result != ResultOptions::BackupAndReplace {
            return Ok(())
        }

        let source = directory.join(path);
        let backup = self.directory.join(&self.configuration.backup).join(path);
        create_parent(&backup)?;
        fs::copy(&source, &backup)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not back up {}", source.display()))?;

        Ok(())
    }

    fn write(&self, directory: &Path, path: &Path, content: &str) -> Result<()> {
        match self.configuration.result {
            ResultOptions::Replace | ResultOptions::BackupAndReplace => write(&directory.join(path), content),
            ResultOptions::CreateNew => write(&self.directory.join(&self.configuration.output).join(path), content)
        }
    }
}


fn spanned(diagnostic: PatchDiagnostic, source: &PatchSource) -> PatchDiagnostic {
    let span = source.spans.fields.get(diagnostic.field).copied();

    PatchDiagnostic { span, ..diagnostic }
}

fn file_diagnostic(patch: &Patch, source: &PatchSource, files: &[&File]) -> Option<PatchDiagnostic> {
    if !patch.enabled { return None }

    let diagnostic = match Pattern::new(&patch.file) {
        Err(error) => PatchDiagnostic::new(
            DiagnosticKind::IncorrectFilePattern, "patch", "file", error.to_string()
        ),
        Ok(pattern) if !files.iter().any(|file| pattern.matches_path(&file.path)) => PatchDiagnostic::new(
            DiagnosticKind::FileNotMatched, "patch", "file",
            format!("'{}' matches no file in the target", patch.file)
        ),
        Ok(_) => return None
    };

    Some(spanned(diagnostic, source))
}

fn file_pattern(name: &str, patch: &Patch) -> Result<Pattern> {
    Pattern::new(&patch.file)
        .into_diagnostic()
        .wrap_err_with(|| format!("Patch {} has an invalid file pattern '{}'", name, patch.file))
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
