// Standard Uses
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

// Crate Uses
use crate::preview::configuration;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::{LocationMatches, Operation};
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::trace::Trace;
use codeshaper::core::shaping::project::settings;
use codeshaper::core::target::File;


const KEPT: usize = 64;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub samples: usize,
    pub lines: usize,
    pub characters: usize
}

impl Default for Limits {
    fn default() -> Self {
        Self { samples: 3, lines: 8, characters: 400 }
    }
}

type Stamps = Vec<(PathBuf, Option<SystemTime>)>;

struct Stamped {
    operation: Operation,
    directory: PathBuf,
    files: Vec<File>,
    stamps: Stamps
}

impl Stamped {
    fn new(registry: &Registry, project: &Path, patch: &Patch) -> Option<Self> {
        let configuration = configuration(project).ok()?;
        let operation = Operation::from_file(&configuration).ok()?;
        let (directory, files) = operation.matching_files(registry, "", patch).ok()?;

        let stamps = [configuration, project.join(settings::FILE)].into_iter()
            .chain(files.iter().map(|file| directory.join(&file.path)))
            .map(|path| { let modified = fs::metadata(&path).and_then(|metadata| metadata.modified()).ok(); (path, modified) })
            .collect();

        Some(Self { operation, directory, files, stamps })
    }
}

struct Counts {
    project: PathBuf,
    patch: Patch,
    stamps: Stamps,
    found: HashMap<String, LocationMatches>
}

#[derive(Default)]
pub struct Counted {
    entries: Vec<Counts>
}

impl Counted {
    pub fn get(
        &mut self, registry: &Registry, project: &Path, patch: &Patch, locations: &[&str], keep_going: &dyn Fn() -> bool
    ) -> Option<Vec<LocationMatches>> {
        let Stamped { operation, directory, files, stamps } = Stamped::new(registry, project, patch)?;
        let known = self.entries.iter().position(|entry| entry.project == project && entry.patch == *patch)
            .map(|index| self.entries.remove(index))
            .filter(|entry| entry.stamps == stamps);
        let mut entry = known.unwrap_or_else(|| Counts {
            project: project.to_path_buf(), patch: patch.clone(), stamps, found: HashMap::new()
        });

        let missing: Vec<&str> = locations.iter().copied().filter(|location| !entry.found.contains_key(*location)).collect();
        let counted = operation.locate_all(registry, patch, (&directory, &files), &missing, keep_going).ok();
        entry.found.extend(missing.iter().map(|location| location.to_string()).zip(counted.into_iter().flatten()));

        let found = locations.iter().map(|location| entry.found.get(*location).copied()).collect();
        if self.entries.len() >= KEPT { self.entries.remove(0); }
        self.entries.push(entry);

        found
    }
}

struct Entry {
    project: PathBuf,
    patch: Patch,
    stamps: Stamps,
    trace: Arc<Trace>
}

#[derive(Default)]
pub struct Traces {
    pub limits: Limits,
    entries: Vec<Entry>,
    guard: Option<Box<dyn Fn() -> bool>>
}

impl Traces {
    pub fn new(limits: Limits) -> Self {
        Self { limits, ..Self::default() }
    }

    pub fn guard(&mut self, keep_going: impl Fn() -> bool + 'static) {
        self.guard = Some(Box::new(keep_going));
    }

    pub fn get(&mut self, registry: &Registry, project: &Path, text: &str) -> Option<Arc<Trace>> {
        let patch = Patch::from_kdl("", text).ok()?;
        let Stamped { operation, stamps, .. } = Stamped::new(registry, project, &patch)?;

        let found = self.entries.iter()
            .find(|entry| entry.project == project && entry.patch == patch && entry.stamps == stamps);
        if let Some(entry) = found {
            return Some(entry.trace.clone())
        }

        let keep_going = || self.guard.as_ref().is_none_or(|guard| guard());
        let trace = Arc::new(operation.trace_while(registry, "", &patch, self.limits.samples, &keep_going).ok()?);

        self.entries.retain(|entry| entry.project != project || entry.patch != patch);
        if self.entries.len() >= KEPT { self.entries.remove(0); }
        self.entries.push(Entry { project: project.to_path_buf(), patch, stamps, trace: trace.clone() });

        Some(trace)
    }
}


pub fn diff(before: &str, after: &str, limits: &Limits) -> String {
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();

    let prefix = before.iter().zip(&after).take_while(|(old, new)| old == new).count();
    let suffix = before[prefix..].iter().rev().zip(after[prefix..].iter().rev()).take_while(|(old, new)| old == new).count();

    let lines = before[..prefix].iter().map(|line| format!("  {line}"))
        .chain(before[prefix..before.len() - suffix].iter().map(|line| format!("- {line}")))
        .chain(after[prefix..after.len() - suffix].iter().map(|line| format!("+ {line}")))
        .chain(before[before.len() - suffix..].iter().map(|line| format!("  {line}")))
        .collect::<Vec<_>>()
        .join("\n");

    fenced("diff", &truncate(&lines, limits))
}

pub fn truncate(text: &str, limits: &Limits) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept = lines.iter().take(limits.lines).copied().collect::<Vec<_>>().join("\n");
    let mut cut = lines.len() > limits.lines;

    if let Some((index, _)) = kept.char_indices().nth(limits.characters) {
        kept.truncate(index);
        cut = true;
    }

    if cut { kept + "\n…" } else { kept }
}

pub fn fenced(language: &str, text: &str) -> String {
    let fence = if text.contains("```") { "````" } else { "```" };

    format!("{fence}{language}\n{text}\n{fence}")
}

pub fn inline(text: &str) -> String {
    let text = text.replace('\n', "⏎").replace('\t', "⇥");
    let fence = if text.contains('`') { "`` " } else { "`" };
    let closing = if text.contains('`') { " ``" } else { "`" };

    format!("{fence}{text}{closing}")
}
