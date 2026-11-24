// Standard Uses
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

// Crate Uses
use crate::document::{kind, DocumentKind};
use crate::position::LineIndex;
use crate::preview::configuration;
use crate::schema::LOCATIONS;
use crate::syntax::Outline;
use crate::target;
use crate::worker::{file_uri, path};

// External Uses
use codeshaper::core::ast::backend::{LanguageBackend, SyntaxNode};
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::operation::Operation;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::project::settings::ProjectSettings;
use codeshaper::core::shaping::project::{patch_files, Project};
use codeshaper::core::target::File;
use dunce::canonicalize;
use lsp_types::{Location, Uri};
use miette::{bail, miette, IntoDiagnostic, Result};
use serde_json::{json, Value};


pub const FILES_REQUEST: &str = "codeshaper/explore";
pub const BLOCK_REQUEST: &str = "codeshaper/exploreBlock";
pub const HINTS_REQUEST: &str = "codeshaper/exploreHints";
pub const PLACE_REQUEST: &str = "codeshaper/explorePlace";
pub const PLACES_REQUEST: &str = "codeshaper/explorePlaces";
pub const REQUESTS: [&str; 5] = [FILES_REQUEST, BLOCK_REQUEST, HINTS_REQUEST, PLACE_REQUEST, PLACES_REQUEST];


pub type Open = HashMap<PathBuf, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explored {
    pub path: PathBuf,
    pub language: String,
    pub usual: Option<String>
}

#[derive(Debug)]
pub struct Files {
    pub directory: PathBuf,
    pub project: PathBuf,
    pub files: Vec<Explored>,
    pub without: usize
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Use {
    pub path: PathBuf,
    pub patch: String,
    pub action: String,
    pub field: String,
    pub enabled: bool,
    pub literal: Range<usize>,
    pub place: Range<usize>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHint {
    pub offset: usize,
    pub parts: Vec<HintPart>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintPart {
    pub node: usize,
    pub label: String,
    pub location: String
}

#[derive(Default)]
pub struct Located {
    text: String,
    found: HashMap<(String, String), Vec<Range<usize>>>
}

struct Asked<'p> {
    text: &'p str,
    file: Explored,
    index: LineIndex<'p>,
    params: &'p Value
}


impl Located {
    fn places(&mut self, backend: &dyn LanguageBackend, text: &str, location: &str) -> Vec<Range<usize>> {
        if self.text != text {
            self.text = text.to_string();
            self.found.clear();
        }

        self.found.entry((backend.name().to_string(), location.to_string()))
            .or_insert_with(|| backend.locate(text, location).unwrap_or_default())
            .clone()
    }
}

impl<'p> Asked<'p> {
    fn new(params: &'p Value) -> Result<Self> {
        let text = params["text"].as_str().ok_or_else(|| miette!("The request has no text"))?;
        let field = |name: &str| params["file"][name].as_str().map(str::to_string);

        Ok(Self {
            text,
            file: Explored {
                path: field("path").map(PathBuf::from).ok_or_else(|| miette!("The request has no file"))?,
                language: field("language").ok_or_else(|| miette!("The request has no language"))?,
                usual: field("usual")
            },
            index: LineIndex::new(text),
            params
        })
    }

    fn backend<'r>(&self, registry: &'r Registry) -> Result<&'r dyn LanguageBackend> {
        backend(registry, &self.file.language)
    }

    fn range(&self) -> Result<Range<usize>> {
        let range: lsp_types::Range = serde_json::from_value(self.params["range"].clone()).into_diagnostic()?;
        let (start, end) = (self.index.offset(range.start), self.index.offset(range.end));

        Ok(start..end.max(start))
    }

    fn location(&self) -> Result<&str> {
        self.params["location"].as_str().ok_or_else(|| miette!("The request has no location"))
    }
}


pub fn answer(
    registry: &Registry, located: &mut Located, method: &str, start: &Path, params: &Value, open: &Open, keep_going: &dyn Fn() -> bool
) -> Result<Value> {
    match method {
        FILES_REQUEST => Ok(listed(&files(registry, start, open)?)),
        BLOCK_REQUEST => {
            let asked = Asked::new(params)?;

            described(registry, located, &asked, start, open)
        }
        HINTS_REQUEST => {
            let asked = Asked::new(params)?;
            let depth = params["depth"].as_u64().unwrap_or_default() as usize;
            let nodes = asked.backend(registry)?.nodes(asked.text)?;
            let found = match params["all"].as_bool() {
                Some(true) => hints(asked.text, &nodes, None),
                _ => hints(asked.text, &nodes, Some(depth))
            };

            Ok(found.into_iter()
                .map(|hint| {
                    let parts: Vec<Value> = hint.parts.into_iter()
                        .map(|part| json!({
                            "node": part.node, "label": part.label, "location": part.location,
                            "range": asked.index.range(nodes[part.node].range.clone())
                        }))
                        .collect();

                    json!({ "position": asked.index.position(hint.offset), "parts": parts })
                })
                .collect())
        }
        PLACE_REQUEST => {
            let asked = Asked::new(params)?;
            let forward = params["forward"].as_bool().unwrap_or(true);

            place(registry, &asked, start, open, forward, keep_going)
        }
        PLACES_REQUEST => {
            let asked = Asked::new(params)?;

            Ok(json!(places(registry, &asked, start, open, keep_going)?))
        }
        method => bail!("Unsupported request '{method}'")
    }
}

pub fn files(registry: &Registry, start: &Path, open: &Open) -> Result<Files> {
    let (configuration, patch) = match kind(start) {
        DocumentKind::Patch { project } => (configuration(&project)?, target::patch(&Outline::parse(&read(start, open)?))),
        DocumentKind::Configuration => (start.to_path_buf(), Patch::default()),
        _ => bail!("{} is not a patch or a configuration", start.display())
    };

    let directory = configuration.parent().unwrap_or(Path::new("")).to_path_buf();
    let configuration = configuration::from_kdl(&read(&configuration, open)?)?;
    let project = directory.join(&configuration.project);
    let operation = Operation {
        configuration, directory,
        project: Project {
            directory: project.to_string_lossy().to_string(),
            settings: ProjectSettings::from_path(&project)?,
            patches: BTreeMap::new(),
            sources: BTreeMap::new()
        }
    };

    let solution = operation.target(registry)?;
    let wanted: Vec<&File> = solution.files().into_iter().filter(|file| patch.file.is_empty() || patch.matches(&file.path)).collect();
    let files: Vec<Explored> = wanted.iter()
        .filter_map(|file| Some(Explored {
            path: file.path.clone(),
            language: operation.backend(registry, &patch, file).ok()?.name().to_string(),
            usual: operation.backend(registry, &Patch::default(), file).ok().map(|backend| backend.name().to_string())
        }))
        .collect();
    let without = wanted.len() - files.len();

    Ok(Files { directory: solution.directory, project, files, without })
}

pub fn block_at(nodes: &[SyntaxNode], range: &Range<usize>) -> Option<usize> {
    let holds = |node: &SyntaxNode| node.range.start <= range.start && range.end <= node.range.end && (range.start < node.range.end || range.end > range.start);

    (0..nodes.len())
        .filter(|node| nodes[*node].parent.is_some() && holds(&nodes[*node]))
        .min_by_key(|node| (nodes[*node].range.len(), std::cmp::Reverse(*node)))
}

pub fn hints(text: &str, nodes: &[SyntaxNode], depth: Option<usize>) -> Vec<BlockHint> {
    let mut depths = vec![0; nodes.len()];
    let mut found: Vec<BlockHint> = vec![];

    for (index, node) in nodes.iter().enumerate() {
        let Some(parent) = node.parent else { continue };
        let is_block = text[node.range.clone()].trim_end().contains('\n');
        depths[index] = depths[parent] + usize::from(is_block);

        let is_wanted = depth.is_none_or(|depth| is_block && depths[index] <= depth);
        if !is_wanted || node.range.is_empty() || node.kind == "ERROR" { continue }

        let label = node.field.as_ref().unwrap_or(&node.kind).clone();
        let location = match &node.field {
            Some(field) if nodes[parent].parent.is_some() => format!("{}.{field}", nodes[parent].kind),
            _ => node.kind.clone()
        };

        let part = HintPart { node: index, label, location };

        match found.last_mut().filter(|hint| hint.offset == node.range.start) {
            Some(hint) => {
                if let Some(outer) = hint.parts.last_mut() { outer.label = nodes[outer.node].kind.clone(); }
                hint.parts.push(part);
            }
            None => found.push(BlockHint { offset: node.range.start, parts: vec![part] })
        }
    }

    found
}

pub fn uses(
    registry: &Registry, located: &mut Located, file: &Explored, text: &str, block: &Range<usize>, project: &Path, open: &Open
) -> Vec<Use> {
    let mut found = vec![];

    for (name, path) in patch_files(&project.join("patches")).unwrap_or_default() {
        let Ok(source) = read(&path, open) else { continue };
        let outline = Outline::parse(&source);
        let patch = target::patch(&outline);
        let language = match patch.alias.as_str() {
            "" => file.usual.clone(),
            alias => Some(alias.to_string())
        };
        let Some(backend) = language.and_then(|language| registry.backend(&language)) else { continue };
        if !patch.matches(&file.path) { continue }

        for (node, literal) in outline.literals().filter(|(node, _)| LOCATIONS.contains(&outline.nodes[*node].name.as_str())) {
            let Some(action) = outline.nodes[node].parent.and_then(|action| outline.action_path(action)) else { continue };
            let smallest = located.places(backend, text, &literal.value).into_iter()
                .filter(|place| place.start <= block.start && block.end <= place.end)
                .min_by_key(|place| place.len());

            found.extend(smallest.map(|place| Use {
                path: path.clone(), patch: name.clone(), action, field: outline.nodes[node].name.clone(),
                enabled: outline.argument("enabled") == Some("true"),
                literal: literal.range.clone(), place
            }));
        }
    }

    found.sort_by_key(|found| found.place.len());
    found
}

fn listed(files: &Files) -> Value {
    let found: Vec<Value> = files.files.iter().filter_map(|file| described_file(files, file)).collect();

    json!({ "files": found, "without": files.without })
}

fn described_file(files: &Files, file: &Explored) -> Option<Value> {
    Some(json!({
        "uri": file_uri(&files.directory.join(&file.path))?.as_str(),
        "path": file.path.to_string_lossy().replace(MAIN_SEPARATOR, "/"),
        "language": file.language,
        "usual": file.usual
    }))
}

fn described(registry: &Registry, located: &mut Located, asked: &Asked, start: &Path, open: &Open) -> Result<Value> {
    let backend = asked.backend(registry)?;
    let nodes = backend.nodes(asked.text)?;
    let range = asked.range()?;
    let named = asked.params["node"].as_u64()
        .map(|node| node as usize)
        .filter(|node| nodes.get(*node).is_some_and(|node| node.parent.is_some() && node.range == range));
    let Some(block) = named.or_else(|| block_at(&nodes, &range)) else { return Ok(Value::Null) };

    let node = &nodes[block];
    let suggestions = backend.suggest_locations(asked.text, block)?;
    let paths: Vec<Value> = suggestions.paths.iter()
        .map(|path| json!({
            "location": path.location,
            "places": path.ranges.len(),
            "ranges": path.ranges.iter().map(|range| asked.index.range(range.clone())).collect::<Vec<_>>(),
            "number": path.ranges.iter().position(|range| *range == node.range).map(|index| index + 1)
        }))
        .collect();

    let parents: Vec<Value> = std::iter::successors(node.parent, |parent| nodes[*parent].parent)
        .map(|parent| &nodes[parent])
        .filter(|parent| parent.parent.is_some())
        .map(|parent| json!({ "kind": parent.kind, "field": parent.field, "range": asked.index.range(parent.range.clone()) }))
        .collect();

    let child = nodes.iter()
        .find(|child| child.parent.is_some_and(|parent| parent >= block) && node.range.start <= child.range.start
            && child.range.end <= node.range.end && child.range != node.range && !child.range.is_empty())
        .map(|child| asked.index.range(child.range.clone()));

    let project = project(start, open);
    let used: Vec<Value> = project.into_iter()
        .flat_map(|project| uses(registry, located, &asked.file, asked.text, &node.range, &project, open))
        .filter_map(|found| {
            let source = read(&found.path, open).ok()?;

            Some(json!({
                "uri": file_uri(&found.path)?.as_str(),
                "patch": found.patch, "action": found.action, "field": found.field, "enabled": found.enabled,
                "range": LineIndex::new(&source).range(found.literal),
                "whole": found.place == node.range
            }))
        })
        .collect();

    Ok(json!({
        "kind": node.kind, "field": node.field, "broken": node.broken, "range": asked.index.range(node.range.clone()),
        "paths": paths, "query": suggestions.query.map(|query| query.location),
        "queryLanguage": backend.query_language().map(|language| json!({ "name": language.name, "documentation": language.documentation })),
        "parents": parents, "child": child, "used": used
    }))
}

fn place(
    registry: &Registry, asked: &Asked, start: &Path, open: &Open, forward: bool, keep_going: &dyn Fn() -> bool
) -> Result<Value> {
    let (backend, location, at) = (asked.backend(registry)?, asked.location()?, asked.range()?);
    backend.check_location(location)?;

    let key = |range: &Range<usize>| (range.start, range.end);
    let here = backend.locate(asked.text, location)?;
    let next = match forward {
        true => here.iter().position(|range| key(range) > key(&at)),
        false => here.iter().rposition(|range| key(range) < key(&at))
    };

    let files = files(registry, start, open)?;
    let own = files.files.iter().position(|file| file.path == asked.file.path);
    let found = |file: &Explored, source: &str, places: &[Range<usize>], index: usize| {
        let mut found = described_file(&files, file).ok_or_else(|| miette!("{} has no address", file.path.display()))?;
        found["range"] = json!(LineIndex::new(source).range(places[index].clone()));
        found["number"] = json!(index + 1);
        found["places"] = json!(places.len());

        Ok(found)
    };
    if let Some(index) = next { return found(&asked.file, asked.text, &here, index) }

    let count = files.files.len();
    for step in 1..=count {
        if !keep_going() { bail!("Stopped") }

        let index = match (own, forward) {
            (Some(own), true) => (own + step) % count,
            (Some(own), false) => (own + count - step) % count,
            (None, true) => step - 1,
            (None, false) => count - step
        };
        let file = &files.files[index];
        if Some(index) == own || file.language != asked.file.language { continue }

        let Ok(source) = fs::read_to_string(files.directory.join(&file.path)) else { continue };
        let places = backend.locate(&source, location)?;
        if places.is_empty() { continue }

        return found(file, &source, &places, if forward { 0 } else { places.len() - 1 })
    }

    match here.len() {
        0 => Ok(Value::Null),
        count => found(&asked.file, asked.text, &here, if forward { 0 } else { count - 1 })
    }
}

fn places(registry: &Registry, asked: &Asked, start: &Path, open: &Open, keep_going: &dyn Fn() -> bool) -> Result<Vec<Location>> {
    let (backend, location) = (asked.backend(registry)?, asked.location()?);
    backend.check_location(location)?;

    let files = files(registry, start, open)?;
    let mut found = vec![];

    for file in files.files.iter().filter(|file| file.language == asked.file.language) {
        if !keep_going() { bail!("Stopped") }

        let path = files.directory.join(&file.path);
        let source = match file.path == asked.file.path {
            true => asked.text.to_string(),
            false => match fs::read_to_string(&path) { Ok(source) => source, Err(_) => continue }
        };
        let Some(uri) = file_uri(&path) else { continue };
        let index = LineIndex::new(&source);

        found.extend(backend.locate(&source, location)?.into_iter().map(|range| Location::new(uri.clone(), index.range(range))));
    }

    Ok(found)
}

fn project(start: &Path, open: &Open) -> Option<PathBuf> {
    match kind(start) {
        DocumentKind::Patch { project } => Some(project),
        DocumentKind::Configuration => {
            let source = read(start, open).ok()?;

            Some(start.parent()?.join(Outline::parse(&source).argument("project")?))
        }
        _ => None
    }
}

fn backend<'r>(registry: &'r Registry, language: &str) -> Result<&'r dyn LanguageBackend> {
    registry.backend(language).ok_or_else(|| miette!("'{language}' is not a known language"))
}

fn read(path: &Path, open: &Open) -> Result<String> {
    match canonicalize(path).ok().and_then(|path| open.get(&path)) {
        Some(text) => Ok(text.clone()),
        None => fs::read_to_string(path).into_diagnostic()
    }
}

pub fn start(params: &Value) -> Option<(Uri, PathBuf)> {
    let uri: Uri = params["uri"].as_str()?.parse().ok()?;
    let path = path(&uri).filter(|path| matches!(kind(path), DocumentKind::Patch { .. } | DocumentKind::Configuration))?;

    Some((uri, path))
}
