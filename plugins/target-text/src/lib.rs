// Relative Modules
pub mod options;

// Standard Uses
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

// Crate Uses
use crate::options::{EXCLUDE, FILES, OPTIONS};

// External Uses
use codeshaper_core::registry::Registry;
use codeshaper_core::target::{File, Group, Options, Solution, TargetLoader};
use glob::{glob_with, MatchOptions, Pattern};
use miette::{bail, Context, IntoDiagnostic, Result};


pub const WILDCARDS: [char; 3] = ['*', '?', '['];
pub const EVERY_FILE: &str = "**/*";


pub fn register(registry: &mut Registry) {
    registry.add_loader(Box::new(TextLoader));
}


pub struct TextLoader;

impl TargetLoader for TextLoader {
    fn name(&self) -> &str { "text" }

    fn options(&self) -> &[&str] { &OPTIONS }

    fn load(&self, path: &Path, options: &Options) -> Result<Solution> {
        if !path.is_dir() {
            bail!("Expected the target to be a folder, got {} instead", path.display());
        }

        let name = path.canonicalize().ok()
            .and_then(|path| Some(path.file_name()?.to_string_lossy().to_string()))
            .unwrap_or_default();
        let files = files(path, options)
            .wrap_err_with(|| format!("Could not list the target at {}", path.display()))?
            .into_iter()
            .map(|path| File { path, language: None })
            .collect();

        Ok(Solution {
            name: name.clone(),
            directory: path.to_path_buf(),
            groups: vec![Group { name, groups: vec![], files }]
        })
    }
}


fn match_options() -> MatchOptions {
    MatchOptions { require_literal_leading_dot: true, ..MatchOptions::new() }
}

pub fn matches<'d>(directory: &'d Path, pattern: &str) -> Result<impl Iterator<Item = Result<PathBuf>> + 'd> {
    let base = Pattern::escape(&directory.to_string_lossy());
    let search = if base.is_empty() {
        pattern.to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), pattern)
    };
    let entries = glob_with(&search, match_options()).into_diagnostic()?;

    Ok(entries
        .filter(|entry| entry.as_ref().map_or(true, |path| path.is_file()))
        .map(move |entry| Ok(entry.into_diagnostic()?.strip_prefix(directory).into_diagnostic()?.to_path_buf())))
}

pub fn excludes(pattern: &str) -> Result<impl Fn(&Path) -> bool> {
    let pattern = Pattern::new(pattern).into_diagnostic()?;

    Ok(move |path: &Path| pattern.matches_path_with(path, match_options()))
}

pub fn is_exact(directory: &Path, entry: &str) -> bool {
    !entry.contains(WILDCARDS) || directory.join(entry).is_file()
}

pub fn plain(entry: &str) -> PathBuf {
    Path::new(entry).components().filter(|part| *part != Component::CurDir).collect()
}

pub fn names(options: &Options) -> Result<&[String]> {
    match options.get(FILES).map(Vec::as_slice) {
        Some([]) => bail!("'files' lists nothing, remove it or use \"{EVERY_FILE}\" to take every file"),
        Some(names) => Ok(names),
        None => Ok(&[])
    }
}

fn files(directory: &Path, options: &Options) -> Result<Vec<PathBuf>> {
    let excluded = options.get(EXCLUDE).into_iter().flatten()
        .map(|pattern| excludes(pattern).wrap_err_with(|| format!("'exclude' has an invalid pattern '{pattern}'")))
        .collect::<Result<Vec<_>>>()?;

    let mut files = vec![];
    let mut seen = HashSet::new();
    let mut add = |found: Vec<PathBuf>| {
        for path in found {
            if seen.insert(path.clone()) && !excluded.iter().any(|excluded| excluded(&path)) {
                files.push(path);
            }
        }
    };

    let names = names(options)?;

    if names.is_empty() {
        add(sorted(directory, EVERY_FILE)?);
    }

    for name in names {
        if is_exact(directory, name) {
            if !directory.join(name).is_file() {
                bail!("'files' lists '{name}', which does not exist");
            }

            add(vec![plain(name)]);
        } else {
            let found = sorted(directory, name)?;

            if found.is_empty() {
                bail!("'files' lists '{name}', which matches no files");
            }

            add(found);
        }
    }

    Ok(files)
}

fn sorted(directory: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let mut found = matches(directory, pattern)
        .wrap_err_with(|| format!("'files' has an invalid pattern '{pattern}'"))?
        .collect::<Result<Vec<_>>>()?;
    found.sort();

    Ok(found)
}
