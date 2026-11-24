// Standard Uses
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

// Crate Uses
use crate::completion::item;
use crate::diagnostics::problem;
use crate::document::{kind, DocumentKind};
use crate::hover::listing;
use crate::options::{self, given, target_kind, targets, Target};
use crate::position::LineIndex;
use crate::syntax::{Entry, Outline};

// External Uses
use codeshaper::core::shaping::patch::source::PatchSource;
use codeshaper::core::target::Options;
use codeshaper_target_text::options::{listed, Field, Listed, FILES};
use codeshaper_target_text::{excludes, is_exact, matches, names, plain, EVERY_FILE, WILDCARDS};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic, DiagnosticSeverity, TextEdit};


type Check<'c> = dyn Fn(&Target, &[Judged], &str) -> Option<Vec<Found>> + 'c;

const TEXT: &str = "text";
const FOLDERS: [&str; 4] = ["project", "target", "output", "backup"];
const REQUIRED: [&str; 2] = ["project", "target"];
const WALKED: usize = 1000;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opening {
    File(PathBuf),
    Pattern(Field, String)
}

#[derive(Debug, Clone)]
struct Judged {
    entry: Listed,
    default: bool
}

#[derive(Debug, Clone, PartialEq)]
struct Found {
    field: Field,
    range: Range<usize>,
    severity: DiagnosticSeverity,
    code: &'static str,
    message: String
}


pub fn completion(path: &Path, text: &str, offset: usize) -> Vec<CompletionItem> {
    let outline = Outline::parse(text);
    let Some(directory) = path.parent() else { return vec![] };
    let Some(entry) = outline.entry(text, offset).filter(|entry| entry.content.start <= offset) else { return vec![] };
    let typed = &text[entry.content.start..offset];
    let (base, segment) = typed.split_at(typed.rfind('/').map(|slash| slash + 1).unwrap_or(0));

    let names = match (field(&outline, entry.node), kind(path)) {
        (Some(field), _) if target_kind(path, text).as_deref() == Some(TEXT) => {
            let current = &text[entry.content.clone()];
            let taken: BTreeSet<&str> = values(&outline).filter(|value| *value != current && field == Field::Files).collect();
            let mut names = BTreeMap::new();

            for target in applicable(path, targets(path, text, Options::new()), field) {
                let literal = target.directory.join(base).is_dir();
                let fixed = base.split_inclusive('/').take_while(|part| literal || !part.contains(WILDCARDS)).collect::<String>();
                let found = if fixed == base { on_disk(&target.directory, base, segment) } else { BTreeMap::new() };

                names.extend(found.into_iter().filter(|(name, folder)| *folder || !taken.contains(format!("{base}{name}").as_str())));
                names.extend(extensions(&target.directory.join(fixed)).into_iter().map(|extension| (format!("*.{extension}"), false)));
                if !base.ends_with("**/") { names.insert("**".to_string(), true); }
            }

            names
        }
        (_, DocumentKind::Configuration) if is_folder(&outline, &entry, offset) => {
            on_disk(directory, base, segment).into_iter().filter(|(_, folder)| *folder).collect()
        }
        _ => return vec![]
    };

    items(text, &entry, offset, names)
}

pub fn target(text: &str, content: &Range<usize>, offset: usize, files: &[PathBuf]) -> Vec<CompletionItem> {
    let line = text[content.clone()].find(['\n', '\r']).map(|length| content.start + length).unwrap_or(content.end);
    if offset > line { return vec![] }

    let typed = &text[content.start..offset];
    let base = &typed[..typed.rfind('/').map(|slash| slash + 1).unwrap_or(0)];

    let names = files.iter()
        .filter_map(|file| {
            let file = file.to_string_lossy().replace(MAIN_SEPARATOR, "/");
            let rest = file.strip_prefix(base)?;

            Some(match rest.split_once('/') {
                Some((folder, _)) => (folder.to_string(), true),
                None => (rest.to_string(), false)
            })
        })
        .collect();

    items(text, &Entry { node: 0, content: content.start..line, quoted: true }, offset, names)
}

pub fn check(path: &Path, text: &str) -> Vec<Diagnostic> {
    let Some(directory) = path.parent() else { return vec![] };
    let index = LineIndex::new(text);
    let mut found = vec![];

    if kind(path) == DocumentKind::Configuration {
        let Ok(source) = PatchSource::parse("configuration.kdl", text) else { return vec![] };
        let outline = Outline::parse(text);

        found.extend(REQUIRED.iter().filter_map(|name| {
            let span = source.spans.fields.get(*name)?;
            let value = outline.argument(name).filter(|value| !directory.join(value).is_dir())?;

            Some(problem(&index, &(span.offset()..span.offset() + span.len()), DiagnosticSeverity::ERROR, "folder-not-found",
                format!("{name}: '{value}' is not a folder in {}", directory.display())))
        }));
    }

    found.extend(judge(path, text, &|target, judged, place| Some(check_names(target, judged, place, kind(path) == DocumentKind::Settings, text))).unwrap_or_default());
    found
}

pub fn check_patterns(path: &Path, text: &str, keep_going: &dyn Fn() -> bool) -> Option<Vec<Diagnostic>> {
    judge(path, text, &|target, judged, place| check_matches(target, judged, place, keep_going))
}

fn judge(path: &Path, text: &str, check: &Check) -> Option<Vec<Diagnostic>> {
    let index = LineIndex::new(text);
    if target_kind(path, text).as_deref() != Some(TEXT) { return Some(vec![]) }
    let Some(given) = given(path, text) else { return Some(vec![]) };
    let entries = listed(text).unwrap_or_default();
    let argument = options::argument(&Outline::parse(text)).unwrap_or_default();
    let targets = targets(path, text, given);
    let mut found: Vec<Found> = vec![];
    let mut times: Vec<usize> = vec![];

    for target in &targets {
        let mut judged: Vec<Judged> = entries.iter()
            .filter(|entry| kind(path) == DocumentKind::Configuration || !target.overrides.contains_key(entry.field.name()))
            .map(|entry| Judged { entry: entry.clone(), default: false })
            .collect();
        if kind(path) == DocumentKind::Configuration {
            for (name, values) in target.defaults.iter().filter(|(name, _)| !target.overrides.contains_key(*name)) {
                let Some(field) = Field::named(name) else { continue };
                judged.extend(values.iter().map(|value| Judged {
                    entry: Listed { field, value: value.clone(), range: argument.clone() }, default: true
                }));
            }
        }

        let place = match targets.len() {
            1 => target.directory.display().to_string(),
            _ => format!("{}, the target of {}", target.directory.display(), target.name())
        };
        for problem in check(target, &judged, &place)? {
            match found.iter().position(|seen| *seen == problem) {
                Some(seen) => times[seen] += 1,
                None => {
                    found.push(problem);
                    times.push(1);
                }
            }
        }
    }

    let everywhere = |field| applicable(path, targets.clone(), field).len();

    Some(found.into_iter().zip(times)
        .filter(|(found, times)| found.severity == DiagnosticSeverity::ERROR || *times >= everywhere(found.field))
        .map(|(found, _)| problem(&index, &found.range, found.severity, found.code, found.message))
        .collect())
}

fn check_names(target: &Target, judged: &[Judged], place: &str, is_settings: bool, text: &str) -> Vec<Found> {
    let outline = Outline::parse(text);
    let argument = options::argument(&outline).unwrap_or_default();
    let options = target.options();
    let from_configuration = target.overrides.contains_key(FILES);
    let mut found = vec![];
    let mut seen = HashSet::new();

    match names(&options) {
        Ok(_) => {}
        Err(_) if from_configuration && is_settings => {}
        Err(error) => found.push(Found {
            field: Field::Files,
            range: options::options(&outline).find(|node| node.name == FILES).map(|node| node.name_range.clone()).unwrap_or(argument),
            severity: DiagnosticSeverity::ERROR, code: "files-empty", message: format!("files: {error}")
        })
    }

    for Judged { entry, default } in judged.iter().filter(|judged| judged.entry.field == Field::Files && is_exact(&target.directory, &judged.entry.value)) {
        let file = target.directory.join(&entry.value);
        let name = plain(&entry.value);
        let mut add = |severity, code, message| if severity == DiagnosticSeverity::ERROR || !default {
            found.push(Found { field: Field::Files, range: entry.range.clone(), severity, code, message });
        };

        if file.is_dir() {
            add(DiagnosticSeverity::ERROR, "file-is-folder", format!("{}: '{}' is a folder", title(entry, *default), entry.value));
        } else if !file.is_file() {
            add(DiagnosticSeverity::ERROR, "file-not-found", format!("{}: '{}' does not exist in {place}", title(entry, *default), entry.value));
        }

        let excluded = options.get(Field::Exclude.name()).into_iter().flatten()
            .find(|exclude| excludes(exclude).is_ok_and(|excluded| excluded(&name)));
        if let Some(exclude) = excluded {
            add(DiagnosticSeverity::WARNING, "excluded-file", format!("files: '{}' is removed by exclude '{exclude}'", entry.value));
        }
        if target.is_own(&name) {
            add(DiagnosticSeverity::WARNING, "own-file", format!("files: '{}' is skipped, it is in one of the operation's own folders", entry.value));
        }
        if !seen.insert(name) {
            add(DiagnosticSeverity::WARNING, "duplicate-file", format!("files: '{}' is already listed", entry.value));
        }
    }

    found
}

fn check_matches(target: &Target, judged: &[Judged], place: &str, keep_going: &dyn Fn() -> bool) -> Option<Vec<Found>> {
    let mut found = vec![];
    let error = |Judged { entry, .. }: &Judged, code, message| Found {
        field: entry.field, range: entry.range.clone(), severity: DiagnosticSeverity::ERROR, code, message
    };

    for judged in judged.iter().filter(|judged| judged.entry.field == Field::Files) {
        let (value, title) = (&judged.entry.value, title(&judged.entry, judged.default));

        match walk(&target.directory, value, keep_going)? {
            Ok(files) if files.is_empty() => found.push(error(judged, "pattern-unmatched", format!("{title}: '{value}' matches no files in {place}"))),
            Ok(_) => {}
            Err(message) => found.push(error(judged, "incorrect-pattern", format!("{title}: {message}")))
        }
    }

    let mut files = None;
    for judged in judged.iter().filter(|judged| judged.entry.field == Field::Exclude) {
        let title = title(&judged.entry, judged.default);

        match excludes(&judged.entry.value) {
            Ok(_) if judged.default => {}
            Ok(excluded) => {
                let files = match &files {
                    Some(files) => files,
                    None => files.insert(every_listed(target, keep_going)?)
                };

                if !files.iter().any(|file| excluded(file)) {
                    found.push(Found {
                        field: Field::Exclude, range: judged.entry.range.clone(), severity: DiagnosticSeverity::HINT,
                        code: "exclude-unmatched", message: format!("exclude: '{}' removes no listed file", judged.entry.value)
                    });
                }
            }
            Err(message) => found.push(error(judged, "incorrect-pattern", format!("{title}: {message}")))
        }
    }

    Some(found)
}

fn title(entry: &Listed, default: bool) -> String {
    match default {
        true => format!("default {}", entry.field.name()),
        false => entry.field.name().to_string()
    }
}

fn applicable(path: &Path, targets: Vec<Target>, field: Field) -> Vec<Target> {
    let is_configuration = kind(path) == DocumentKind::Configuration;

    targets.into_iter().filter(|target| is_configuration || !target.overrides.contains_key(field.name())).collect()
}

pub fn opening(path: &Path, text: &str, offset: usize) -> Option<(Range<usize>, Opening)> {
    let directory = path.parent()?;
    let outline = Outline::parse(text);
    let entry = outline.entry(text, offset).filter(|entry| !entry.content.is_empty())?;
    let value = text[entry.content.clone()].to_string();

    let opening = match (field(&outline, entry.node), kind(path)) {
        (Some(field), _) if target_kind(path, text).as_deref() == Some(TEXT) => {
            let targets = applicable(path, targets(path, text, Options::new()), field);
            let file = targets.iter().map(|target| target.directory.join(&value)).find(|file| field == Field::Files && file.is_file());

            match file {
                Some(file) => Opening::File(file),
                None if value.contains(WILDCARDS) || field == Field::Exclude => Opening::Pattern(field, value),
                None => return None
            }
        }
        (_, DocumentKind::Configuration) if is_folder(&outline, &entry, offset) => {
            let folder = directory.join(&value);
            let name = match outline.nodes[entry.node].name.as_str() {
                "project" => "settings.kdl",
                "target" => "project.godot",
                _ => return None
            };

            Opening::File(Some(folder.join(name)).filter(|file| file.is_file())?)
        }
        _ => return None
    };

    Some((entry.content, opening))
}

pub fn open(path: &Path, text: &str, field: Field, pattern: &str, keep_going: &dyn Fn() -> bool) -> Vec<PathBuf> {
    let mut files = BTreeSet::new();

    for target in applicable(path, targets(path, text, Options::new()), field) {
        let Some(found) = walk(&target.directory, pattern, keep_going) else { return vec![] };

        files.extend(found.unwrap_or_default().into_iter()
            .filter(|file| !target.is_own(file))
            .map(|file| target.directory.join(file))
            .filter(|file| file.is_file()));
    }

    files.into_iter().collect()
}

pub fn hover(path: &Path, text: &str, offset: usize, keep_going: &dyn Fn() -> bool) -> Option<(String, Range<usize>)> {
    if target_kind(path, text).as_deref() != Some(TEXT) { return None }
    let targets = targets(path, text, given(path, text)?);
    let outline = Outline::parse(text);

    let named = options::options(&outline)
        .filter(|node| node.name_range.start <= offset && offset <= node.name_range.end)
        .find_map(|node| Some((Field::named(&node.name)?, node.name_range.clone())));
    if let Some((field, range)) = named {
        return Some((origin(path, &targets, field), range))
    }

    let entries = listed(text).ok()?;
    let entry = entries.iter().find(|entry| entry.range.start <= offset && offset <= entry.range.end)?;
    let applicable = applicable(path, targets, entry.field);
    let mut values = vec![];

    for target in &applicable {
        let Some(value) = explain(target, entry, keep_going)? else { continue };

        values.push(match applicable.len() {
            1 => value,
            _ => format!("**{}**: {value}", target.name())
        });
    }

    (!values.is_empty()).then(|| (values.join("\n\n"), entry.range.clone()))
}

fn origin(path: &Path, targets: &[Target], field: Field) -> String {
    let name = field.name();

    match kind(path) {
        DocumentKind::Configuration => {
            match targets.iter().find_map(|target| target.defaults.get(name)) {
                Some(values) if values.is_empty() => format!("Used instead of the project's empty `{name}`"),
                Some(values) => {
                    let values: Vec<String> = values.iter().map(|value| format!("- `{value}`")).collect();
                    format!("Used instead of the project's `{name}`:\n\n{}", values.join("\n"))
                }
                None => format!("The project has no default `{name}`")
            }
        }
        _ => {
            let replaced: Vec<String> = targets.iter()
                .filter(|target| target.overrides.contains_key(name))
                .map(|target| format!("`{}`", target.name()))
                .collect();

            match replaced.is_empty() {
                true => "Used for every target".to_string(),
                false => format!("Used for every target, except the ones of {} that have their own", replaced.join(", "))
            }
        }
    }
}

fn explain(target: &Target, entry: &Listed, keep_going: &dyn Fn() -> bool) -> Option<Option<String>> {
    Some(Some(match entry.field {
        Field::Files if is_exact(&target.directory, &entry.value) => return Some(None),
        Field::Files => match walk(&target.directory, &entry.value, keep_going)? {
            Ok(files) => {
                let files: Vec<PathBuf> = files.into_iter().filter(|file| !target.is_own(file)).collect();

                match files.is_empty() {
                    true => "Matches no file".to_string(),
                    false => listing(format!("Matches {} file(s)", files.len()), &target.directory, &files)
                }
            }
            Err(message) => message
        },
        Field::Exclude => match excludes(&entry.value) {
            Ok(excluded) => {
                let listed = every_listed(target, keep_going)?;
                let removed: Vec<PathBuf> = listed.iter().filter(|file| excluded(file)).cloned().collect();

                match removed.is_empty() {
                    true => "Removes no listed file".to_string(),
                    false => listing(format!("Removes {} of {} listed file(s)", removed.len(), listed.len()), &target.directory, &removed)
                }
            }
            Err(error) => error.to_string()
        }
    }))
}

pub fn walk(directory: &Path, pattern: &str, keep_going: &dyn Fn() -> bool) -> Option<Result<Vec<PathBuf>, String>> {
    if is_exact(directory, pattern) { return Some(Ok(vec![plain(pattern)])) }

    let entries = match matches(directory, pattern) {
        Ok(entries) => entries,
        Err(error) => return Some(Err(error.to_string()))
    };
    let mut files = vec![];

    for entry in entries {
        if !keep_going() { return None }

        match entry {
            Ok(file) => files.push(file),
            Err(error) => return Some(Err(error.to_string()))
        }
    }

    Some(Ok(files))
}

fn every_listed(target: &Target, keep_going: &dyn Fn() -> bool) -> Option<BTreeSet<PathBuf>> {
    let options = target.options();
    let names = match names(&options) {
        Ok([]) => vec![EVERY_FILE.to_string()],
        Ok(names) => names.to_vec(),
        Err(_) => vec![]
    };
    let mut files = BTreeSet::new();

    for name in names {
        files.extend(walk(&target.directory, &name, keep_going)?.unwrap_or_default());
    }
    files.retain(|file| !target.is_own(file));

    Some(files)
}

fn field(outline: &Outline, node: usize) -> Option<Field> {
    let parent = outline.nodes[node].parent?;

    Field::named(&outline.nodes[node].name).filter(|_| outline.is_top(parent, options::TARGET))
}

fn is_folder(outline: &Outline, entry: &Entry, offset: usize) -> bool {
    let node = &outline.nodes[entry.node];
    let in_block = node.children.as_ref().is_some_and(|children| children.start <= offset && offset <= children.end);

    node.parent.is_none() && !in_block && FOLDERS.contains(&node.name.as_str())
}

fn values(outline: &Outline) -> impl Iterator<Item = &str> {
    let lists = |node: usize| field(outline, node) == Some(Field::Files);

    outline.nodes.iter().enumerate().flat_map(move |(position, node)| {
        let arguments = node.arguments.iter().filter(move |_| lists(position)).map(|literal| literal.value.as_str());

        arguments.chain(node.parent.filter(|parent| lists(*parent)).map(|_| node.name.as_str()))
    })
}

fn on_disk(directory: &Path, base: &str, segment: &str) -> BTreeMap<String, bool> {
    let entries = fs::read_dir(directory.join(base)).into_iter().flatten();

    entries.flatten()
        .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path().is_dir())))
        .filter(|(name, _)| segment.starts_with('.') || !name.starts_with('.'))
        .filter(|(name, _)| !name.contains(['"', '\\']))
        .collect()
}

fn extensions(directory: &Path) -> BTreeSet<String> {
    let mut folders = VecDeque::from([directory.to_path_buf()]);
    let mut found = BTreeSet::new();
    let mut walked = 0;

    while let Some(folder) = folders.pop_front() {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            walked += 1;
            if walked > WALKED { return found }
            if entry.file_name().to_string_lossy().starts_with('.') { continue }

            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                folders.push_back(path);
            } else if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
                found.insert(extension.to_string());
            }
        }
    }

    found
}

fn items(text: &str, entry: &Entry, offset: usize, names: BTreeMap<String, bool>) -> Vec<CompletionItem> {
    let index = LineIndex::new(text);
    let typed = &text[entry.content.start..offset];
    let (base, _) = typed.split_at(typed.rfind('/').map(|slash| slash + 1).unwrap_or(0));
    let rest = &text[offset..entry.content.end];
    let next = rest.find('/');

    names.into_iter()
        .map(|(name, folder)| {
            let label = if folder { format!("{name}/") } else { name };
            let end = match (folder, next) {
                (true, Some(next)) => offset + next + 1,
                _ => entry.content.end
            };
            let edit = match entry.quoted {
                true => TextEdit::new(index.range(entry.content.start + base.len()..end), label.clone()),
                false => TextEdit::new(index.range(entry.content.start..end), format!("\"{base}{label}\""))
            };

            CompletionItem {
                text_edit: Some(CompletionTextEdit::Edit(edit)),
                ..item(&label, if folder { CompletionItemKind::FOLDER } else { CompletionItemKind::FILE }, None)
            }
        })
        .collect()
}
