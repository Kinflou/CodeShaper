// Standard Uses
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

// Crate Uses

// External Uses
use clap::{Parser, Subcommand};
use codeshaper::core::shaping::operation::{patch_backend, Operation};
use codeshaper::core::shaping::operation::report::Outcome;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::controller::PatchController;
use codeshaper::core::shaping::patch::source::PatchSource;
use codeshaper::core::shaping::patch::validation::{validate_source, Diagnostic, Severity};
use codeshaper::registry;
use miette::{miette, Context, IntoDiagnostic, MietteHandlerOpts, Result};
use similar::TextDiff;


const VERSION: &str = match option_env!("CODESHAPER_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION")
};


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
        dry_run: bool
    },
    #[command(about = "Check an operation, or single patch files, for problems")]
    Check {
        #[arg(help = "The operation's configuration.kdl, or patch files with --patch")]
        paths: Vec<PathBuf>,
        #[arg(long, help = "Check the given paths as patch files")]
        patch: bool
    },
    #[command(about = "Apply one patch to one file")]
    Apply {
        #[arg(help = "The patch file")]
        patch: PathBuf,
        #[arg(help = "The file to shape")]
        file: PathBuf,
        #[arg(long, help = "Write the result into the file instead of showing a diff")]
        in_place: bool
    },
    #[command(about = "Start the language server over stdio")]
    Lsp
}


fn main() -> Result<ExitCode> {
    miette::set_hook(Box::new(|_| Box::new(MietteHandlerOpts::new().wrap_lines(false).build())))?;

    match Cli::parse().command {
        Command::Run { configuration, dry_run: false } => run(&configuration),
        Command::Run { configuration, dry_run: true } => dry_run(&configuration),
        Command::Check { paths, patch: false } => check_operation(&paths),
        Command::Check { paths, patch: true } => check_patches(&paths),
        Command::Apply { patch, file, in_place } => apply(&patch, &file, in_place),
        Command::Lsp => codeshaper_lsp::run().map(|_| ExitCode::SUCCESS)
    }
}


fn run(configuration: &Path) -> Result<ExitCode> {
    let operation = Operation::from_file(configuration)?;
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

fn dry_run(configuration: &Path) -> Result<ExitCode> {
    let operation = Operation::from_file(configuration)?;
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

fn check_operation(paths: &[PathBuf]) -> Result<ExitCode> {
    let [configuration] = paths else {
        return Err(miette!("Expected one configuration file, or patch files with --patch"))
    };

    let operation = Operation::from_file(configuration)?;
    let found = operation.check(&registry())?;

    Ok(report_diagnostics(found, &operation.project.sources))
}

fn check_patches(paths: &[PathBuf]) -> Result<ExitCode> {
    let registry = registry();
    let mut found = vec![];
    let mut sources = BTreeMap::new();

    for path in paths {
        let (patch, source) = Patch::load(path)?;
        let backend = patch_backend(&registry, &patch, "");
        let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();

        found.extend(validate_source(&patch, &source, backend).into_iter().map(|diagnostic| (name.clone(), diagnostic)));
        sources.insert(name, source);
    }

    Ok(report_diagnostics(found, &sources))
}

fn apply(patch_path: &Path, file: &Path, in_place: bool) -> Result<ExitCode> {
    let registry = registry();
    let (patch, source) = Patch::load(patch_path)?;

    let backend = match patch.alias.as_str() {
        "" => registry.backend_for(file),
        alias => registry.backend(alias)
    }.ok_or_else(|| miette!("No language backend for {} with patch alias '{}'", file.display(), patch.alias))?;

    let name = patch_path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let diagnostics: Vec<_> = validate_source(&patch, &source, Some(backend)).into_iter()
        .map(|diagnostic| (name.clone(), diagnostic))
        .collect();
    if diagnostics.iter().any(|(_, diagnostic)| diagnostic.severity == Severity::Error) {
        return Ok(report_diagnostics(diagnostics, &BTreeMap::from([(name, source)])))
    }

    let source = fs::read_to_string(file)
        .into_diagnostic()
        .wrap_err_with(|| format!("Could not read {}", file.display()))?;
    let result = PatchController::with(Rc::new(patch)).apply(backend, &source)?;

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

fn report_diagnostics(found: Vec<(String, Diagnostic)>, sources: &BTreeMap<String, PatchSource>) -> ExitCode {
    let mut errors = 0;
    let mut warnings = 0;

    for (patch, diagnostic) in &found {
        let severity = match diagnostic.severity {
            Severity::Error => { errors += 1; "error" }
            Severity::Warning => { warnings += 1; "warning" }
        };

        let title = format!(
            "{severity}[{}] {patch} › {} › {}: {}",
            diagnostic.kind.code(), diagnostic.action, diagnostic.field, diagnostic.message
        );

        match sources.get(patch).filter(|_| diagnostic.span.is_some()) {
            Some(source) => println!("{:?}", source.report(diagnostic, title)),
            None => println!("{title}")
        }
    }

    println!("{errors} error(s), {warnings} warning(s)");

    if errors > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
