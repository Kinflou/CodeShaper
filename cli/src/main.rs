// Standard Uses
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

// Crate Uses

// External Uses
use clap::{Parser, Subcommand};
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::operation::report::Outcome;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::controller::PatchController;
use codeshaper::core::shaping::patch::file::NamedPatch;
use codeshaper::core::shaping::patch::source::PatchSource;
use codeshaper::core::shaping::patch::validation::{called, validate_source, Diagnostic, DiagnosticKind, Severity};
use codeshaper::core::shaping::project::settings::ProjectSettings;
use codeshaper::core::shaping::project::values::{check_set, is_name, Declared, Source, Values};
use codeshaper::core::shaping::project::Project;
use codeshaper::registry;
use miette::{bail, miette, Context, IntoDiagnostic, MietteHandlerOpts, Result};
use similar::TextDiff;


const VERSION: &str = match option_env!("CODESHAPER_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION")
};


const SET: &str = "Set a variable of the project, can be given several times";


#[derive(Parser)]
#[command(name = "codeshaper", version = VERSION, about = "Shape code with patches over syntax trees")]
struct Cli {
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Run an operation from its configuration file")]
    Run {
        #[arg(help = "The operation's configuration.kdl")]
        configuration: PathBuf,
        #[arg(long, help = "Show the changes as diffs without writing anything")]
        dry_run: bool,
        #[arg(long, value_name = "NAME=VALUE", value_parser = value, help = SET)]
        set: Vec<(String, String)>
    },
    #[command(about = "Check an operation, or single patch files, for problems")]
    Check {
        #[arg(help = "The operation's configuration.kdl, or patch files with --patch")]
        paths: Vec<PathBuf>,
        #[arg(long, help = "Check the given paths as patch files")]
        patch: bool,
        #[arg(long, requires = "patch", help = "The language of patches that don't name one")]
        language: Option<String>,
        #[arg(long, value_name = "NAME=VALUE", value_parser = value, help = SET)]
        set: Vec<(String, String)>
    },
    #[command(about = "Apply one patch to one file")]
    Apply {
        #[arg(help = "The patch file")]
        patch: PathBuf,
        #[arg(help = "The file to shape")]
        file: PathBuf,
        #[arg(long, help = "The patch's name, when the patch file has several")]
        name: Option<String>,
        #[arg(long, help = "Write the result into the file instead of showing a diff")]
        in_place: bool,
        #[arg(long, help = "The file's language, when the patch doesn't name one")]
        language: Option<String>,
        #[arg(long, value_name = "NAME=VALUE", value_parser = value, help = SET)]
        set: Vec<(String, String)>
    },
    #[command(about = "Start the language server over stdio")]
    Lsp
}


fn main() -> Result<ExitCode> {
    miette::set_hook(Box::new(|_| Box::new(MietteHandlerOpts::new().wrap_lines(false).build())))?;

    match Cli::parse().command {
        Command::Run { configuration, dry_run: false, set } => run(&operation(&configuration, &set)?),
        Command::Run { configuration, dry_run: true, set } => dry_run(&operation(&configuration, &set)?),
        Command::Check { paths, patch: false, set, .. } => check_operation(&paths, &set),
        Command::Check { paths, patch: true, language, set } => check_patches(&paths, language.as_deref(), &set),
        Command::Apply { patch, file, name, in_place, language, set } => {
            apply(&patch, &file, name.as_deref(), in_place, language.as_deref(), &set)
        }
        Command::Lsp => codeshaper_lsp::run().map(|_| ExitCode::SUCCESS)
    }
}


fn value(given: &str) -> Result<(String, String), String> {
    match given.split_once('=') {
        Some((name, value)) if is_name(name) => Ok((name.to_string(), value.to_string())),
        Some((name, _)) => Err(format!("'{name}' can't be a variable's name")),
        None => Err("expected NAME=VALUE".to_string())
    }
}

fn operation(configuration: &Path, set: &[(String, String)]) -> Result<Operation> {
    let mut operation = Operation::from_file(configuration)?;

    for (name, value) in set {
        operation.set(name, value)?;
    }

    Ok(operation)
}

fn declared(patch: &Path, set: &[(String, String)]) -> Result<Vec<Declared>> {
    let given: BTreeMap<String, String> = set.iter().cloned().collect();
    let patch = fs::canonicalize(patch).into_diagnostic()?;

    match Project::containing(&patch) {
        Some(project) => Operation::declared_for(&project, &given),
        None => Ok(given.into_iter().map(|(name, value)| Declared { name, value: Some(value), source: Source::Set }).collect())
    }
}

fn run(operation: &Operation) -> Result<ExitCode> {
    let report = operation.run(&registry())?;

    let mut unchanged = 0;
    for file in &report.files {
        match &file.outcome {
            Outcome::Patched(patches) => println!("patched    {}  ({})", file.path.display(), patches.join(", ")),
            Outcome::Unchanged => unchanged += 1
        }
    }

    println!("{} patched, {} unchanged", report.files.len() - unchanged, unchanged);

    Ok(ExitCode::SUCCESS)
}

fn dry_run(operation: &Operation) -> Result<ExitCode> {
    let registry = registry();
    let preview = operation.session(&registry)?.preview()?;

    for file in &preview.files {
        print_diff(&file.path, &file.before, &file.result);
    }

    for (_, failure) in &preview.errors {
        eprintln!("{failure:?}");
    }

    println!("{} file(s) would change, nothing was written", preview.files.len());

    Ok(if preview.errors.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

fn check_operation(paths: &[PathBuf], set: &[(String, String)]) -> Result<ExitCode> {
    let [configuration] = paths else {
        return Err(miette!("Expected one configuration file, or patch files with --patch"))
    };

    let operation = operation(configuration, set)?;
    let found = operation.check(&registry())?;

    Ok(report_diagnostics(found.iter().map(|(name, diagnostic)| {
        (name.as_str(), diagnostic, operation.project.patch(name).map(|patch| &patch.source))
    })))
}

fn check_patches(paths: &[PathBuf], language: Option<&str>, set: &[(String, String)]) -> Result<ExitCode> {
    let registry = registry();
    let mut found = vec![];

    for path in paths {
        let patches = Patch::load_all(path)?;
        let declared = declared(path, set)?;

        for named in patches {
            let NamedPatch { patch, source, .. } = &named;

            let (backends, mut diagnostics) = match languages(&registry, path, patch, language)? {
                Languages::Known(backend) => (vec![backend], vec![]),
                Languages::Mixed(operation) => {
                    let solution = operation.target(&registry)?;
                    let diagnostics = operation.check_file(&registry, patch, source)?;

                    (operation.languages(&registry, patch, &solution.files()).0, diagnostics)
                }
                Languages::Unknown => {
                    let of = named.name.as_ref().map(|name| format!("'{name}' in ")).unwrap_or_default();
                    eprintln!("The locations of {of}{} weren't checked: it is in no project and names no language, use --language", path.display());
                    (vec![], vec![])
                }
            };
            diagnostics.retain(|diagnostic| diagnostic.kind == DiagnosticKind::LanguageNotFound);
            diagnostics.extend(validate_source(patch, source, backends, &declared));

            found.push((display_name(path, &named), named, diagnostics));
        }
    }

    Ok(report_diagnostics(found.iter().flat_map(|(name, named, diagnostics)| {
        diagnostics.iter().map(|diagnostic| (name.as_str(), diagnostic, Some(&named.source)))
    })))
}

fn display_name(path: &Path, named: &NamedPatch) -> String {
    let file = || path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();

    named.name.clone().unwrap_or_else(file)
}

fn chosen(path: &Path, name: Option<&str>) -> Result<NamedPatch> {
    let mut patches = Patch::load_all(path)?;
    let names: Vec<String> = patches.iter().map(|named| display_name(path, named)).collect();

    let index = match name {
        Some(name) => names.iter().position(|known| known == name)
            .ok_or_else(|| miette!("{} has no patch named '{}', only: {}", path.display(), name, names.join(", ")))?,
        None if patches.len() > 1 => bail!("{} has several patches, name one with --name: {}", path.display(), names.join(", ")),
        None => 0
    };

    Ok(patches.swap_remove(index))
}

enum Languages<'r> {
    Known(&'r dyn LanguageBackend),
    Mixed(Box<Operation>),
    Unknown
}

fn languages<'r>(registry: &'r Registry, path: &Path, patch: &Patch, given: Option<&str>) -> Result<Languages<'r>> {
    let known = |name: &str| registry.backend(name).map(Languages::Known).ok_or_else(|| {
        miette!("'{}' is not a known language, known languages are: {}", name, registry.languages().join(", "))
    });

    match (patch.alias.as_str(), given) {
        ("", Some(given)) => return known(given),
        ("", None) => {}
        (alias, Some(given)) if alias != given => bail!("{} names the language '{}', not '{}'", path.display(), alias, given),
        (alias, _) => return known(alias)
    }

    let path = fs::canonicalize(path).into_diagnostic()?;
    let Some(project) = Project::containing(&path) else { return Ok(Languages::Unknown) };
    let ast_set = ProjectSettings::from_path(&project.join(""))?.ast_set;

    ast_set.check(registry)?;

    match ast_set.fixed() {
        Some(name) => known(name),
        None => Operation::for_project(&project).map(|operation| Languages::Mixed(Box::new(operation)))
    }
}

fn target_language<'r>(registry: &'r Registry, operation: &Operation, patch: &Patch, file: &Path) -> Result<&'r dyn LanguageBackend> {
    let path = fs::canonicalize(file).into_diagnostic().wrap_err_with(|| format!("Could not read {}", file.display()))?;
    let solution = operation.target(registry)?;

    let found = solution.files().into_iter()
        .find(|file| fs::canonicalize(solution.directory.join(&file.path)).is_ok_and(|found| found == path))
        .ok_or_else(|| miette!("{} is not in the project's target, name its language with --language", file.display()))?;

    operation.backend(registry, patch, found)
}

fn apply(
    patch_path: &Path, file: &Path, name: Option<&str>, in_place: bool, language: Option<&str>, set: &[(String, String)]
) -> Result<ExitCode> {
    let registry = registry();
    let named = chosen(patch_path, name)?;
    let name = display_name(patch_path, &named);
    let NamedPatch { patch, source, .. } = named;

    let backend = match languages(&registry, patch_path, &patch, language)? {
        Languages::Known(backend) => backend,
        Languages::Mixed(operation) => target_language(&registry, &operation, &patch, file)?,
        Languages::Unknown => bail!("{} is in no project and names no language, use --language", patch_path.display())
    };

    let declared = declared(patch_path, set)?;
    let diagnostics = validate_source(&patch, &source, [backend], &declared);
    if diagnostics.iter().any(|diagnostic| diagnostic.severity == Severity::Error) {
        return Ok(report_diagnostics(diagnostics.iter().map(|diagnostic| (name.as_str(), diagnostic, Some(&source)))))
    }
    check_set(&declared, &called(&patch))?;

    let values: Rc<Values> = Rc::new(declared.as_slice().into());
    let source = fs::read_to_string(file)
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not read {}", file.display()))?;
    let result = PatchController::with(Rc::new(patch)).with_values(values).apply(backend, &source)?;

    if result == source {
        println!("No changes");
    } else if in_place {
        fs::write(file, &result)
            .into_diagnostic()
            .wrap_err_with(|| format!("Could not write {}", file.display()))?;
        println!("Wrote {}", file.display());
    } else {
        print_diff(file, &source, &result);
    }

    Ok(ExitCode::SUCCESS)
}


fn print_diff(path: &Path, old: &str, new: &str) {
    let path = path.display().to_string();
    let diff = TextDiff::from_lines(old, new);

    print!("{}", diff.unified_diff().header(&format!("a/{path}"), &format!("b/{path}")));
}

fn report_diagnostics<'f>(found: impl Iterator<Item = (&'f str, &'f Diagnostic, Option<&'f PatchSource>)>) -> ExitCode {
    let mut errors = 0;
    let mut warnings = 0;

    for (patch, diagnostic, source) in found {
        let severity = match diagnostic.severity {
            Severity::Error => { errors += 1; "error" }
            Severity::Warning => { warnings += 1; "warning" }
        };

        let title = format!(
            "{severity}[{}] {patch} › {} › {}: {}",
            diagnostic.kind.code(), diagnostic.action, diagnostic.field, diagnostic.message
        );

        match source.filter(|_| diagnostic.span.is_some()) {
            Some(source) => println!("{:?}", source.report(diagnostic, title)),
            None => println!("{title}")
        }
    }

    println!("{errors} error(s), {warnings} warning(s)");

    if errors > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
