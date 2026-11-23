// Standard Uses
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

// Crate Uses
use crate::preview::configuration;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::trace::Trace;


const KEPT: usize = 16;


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

struct Entry {
    project: PathBuf,
    text: String,
    stamps: Vec<(PathBuf, Option<SystemTime>)>,
    trace: Rc<Trace>
}

#[derive(Default)]
pub struct Traces {
    pub limits: Limits,
    entries: Vec<Entry>
}

impl Traces {
    pub fn new(limits: Limits) -> Self {
        Self { limits, entries: vec![] }
    }

    pub fn cached(&self, project: &Path, text: &str) -> Option<Rc<Trace>> {
        self.entries.iter()
            .rev()
            .find(|entry| entry.project == project && entry.text == text)
            .map(|entry| entry.trace.clone())
    }

    pub fn get(&mut self, registry: &Registry, project: &Path, text: &str) -> Option<Rc<Trace>> {
        let patch = Patch::from_kdl("", text).ok()?;
        let operation = Operation::from_file(&configuration(project).ok()?).ok()?;
        let (directory, files) = operation.matching_files(registry, "", &patch).ok()?;

        let stamps: Vec<(PathBuf, Option<SystemTime>)> = files.iter()
            .map(|file| (file.path.clone(), fs::metadata(directory.join(&file.path)).and_then(|metadata| metadata.modified()).ok()))
            .collect();

        let found = self.entries.iter()
            .find(|entry| entry.project == project && entry.text == text && entry.stamps == stamps);
        if let Some(entry) = found {
            return Some(entry.trace.clone())
        }

        let trace = Rc::new(operation.trace(registry, "", &patch, self.limits.samples).ok()?);

        self.entries.retain(|entry| entry.project != project || entry.text != text);
        if self.entries.len() >= KEPT { self.entries.remove(0); }
        self.entries.push(Entry { project: project.to_path_buf(), text: text.to_string(), stamps, trace: trace.clone() });

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

pub fn groups(pattern: &str) -> Vec<String> {
    let bytes = pattern.as_bytes();
    let mut open = vec![];
    let mut found: Vec<(usize, usize)> = vec![];
    let mut class = false;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'[' if !class => class = true,
            b']' if class => class = false,
            b'(' if !class => {
                let capturing = bytes.get(index + 1) != Some(&b'?')
                    || pattern[index..].starts_with("(?P<")
                    || (pattern[index..].starts_with("(?<") && !pattern[index..].starts_with("(?<=") && !pattern[index..].starts_with("(?<!"));

                if capturing {
                    found.push((index, index));
                    open.push(Some(found.len() - 1));
                } else {
                    open.push(None);
                }
            }
            b')' if !class => {
                if let Some(Some(group)) = open.pop() {
                    found[group].1 = index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }

    found.into_iter().map(|(start, end)| pattern[start..end.max(start)].to_string()).collect()
}
