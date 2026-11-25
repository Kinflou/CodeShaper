// Standard Uses
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Crate Uses
use crate::explain;
use crate::groups::{arguments, group_number, groups, locals};
use crate::navigation::{calls, symbol_at, Symbol};
use crate::places::Places;
use crate::query::{is_query, part_at, Part, PREDICATES};
use crate::schema::{regex_field, LOCATIONS};
use crate::section::{view_at, View};
use crate::syntax::{Literal, Outline, ACTION_NODES};
use crate::target::{self, backends};
use crate::trace::{diff, fenced, inline, truncate, Limits, Traces};
use crate::values;

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::{LocationMatches, LocationPlaces};
use codeshaper::core::shaping::patch::pattern::{Group, FLAGS};
use codeshaper::core::shaping::patch::trace::{ActionTrace, Found, Trace};
use codeshaper::core::shaping::project::ast_set::AstSet;
use codeshaper::core::shaping::project::settings::from_kdl;
use url::Url;


const LISTED: usize = 10;
const ARGUMENTS: &str = "first argument(s)";


pub fn hover(
    registry: &Registry, project: &Path, traces: &mut Traces, text: &str, offset: usize
) -> Option<(String, Range<usize>)> {
    let View { text, .. } = view_at(text, offset);
    let text = text.as_ref();
    let outline = Outline::parse(text);
    if let Some(explained) = explain::hover(explain::PATCH, &outline, offset) { return Some(explained) }

    let language = backends(registry, project, &outline).first().map(|backend| backend.name().to_string()).unwrap_or_default();

    if let Some(Symbol { name, range }) = symbol_at(&outline, text, offset) {
        let index = outline.nodes.iter().position(|node| {
            ACTION_NODES.contains(&node.name.as_str()) && node.arguments.first().is_some_and(|literal| literal.value == name)
        });
        let Some(index) = index else { return Some((values::hover(project, &name)?, range)) };

        if outline.nodes[index].name == "replacer" {
            let (trace, path) = traced(registry, project, traces, text, &outline, index)?;
            return Some((replacements(trace.action(&path), &trace, &traces.limits), range))
        }

        let sites = calls(&outline, text).iter().filter(|call| call.name == name).count();
        let mut value = format!("Called from {sites} place(s) in this patch");
        if let Some((trace, path)) = traced(registry, project, traces, text, &outline, index) {
            value += &format!("\n\n{}", results(trace.action(&path), &trace, &language, &traces.limits));
        }

        return Some((value, range))
    }

    let (node, literal) = outline.literal_at(offset).map(|(node, literal)| (node, literal.clone()))?;
    let field = outline.nodes[node].name.as_str();

    if LOCATIONS.contains(&field) {
        return location(registry, project, traces, &outline, text, literal, offset)
    }

    if outline.nodes[node].parent.is_none() {
        let value = match field {
            "file" => files(registry, project, &outline)?,
            "alias" => alias(registry, &literal.value),
            _ => return None
        };
        return Some((value, literal.range))
    }

    let action = outline.nodes[node].parent?;
    let kind = outline.nodes[action].name.as_str();

    let passed = arguments(text, &literal).into_iter().find(|(_, range)| range.start <= offset && offset <= range.end);
    if let Some((variable, range)) = passed.filter(|_| matches!((kind, field), ("maker", "make") | ("list" | "switch", "index" | "cases" | "values" | "default"))) {
        let traced = traced(registry, project, traces, text, &outline, action);
        let calls = traced.as_ref().and_then(|(trace, path)| trace.action(path));

        return Some((argument(&variable, outline.nodes[action].arguments.first()?.value.as_str(), calls), range))
    }

    let used = locals(text, &literal).into_iter().find(|(_, range)| range.start <= offset && offset <= range.end);
    if let Some((variable, range)) = used {
        let pattern = outline.field(action, regex_field(kind)?).map(|literal| literal.value.clone()).unwrap_or_default();
        let groups = groups(&outline, action);
        let trace = traced(registry, project, traces, text, &outline, action);
        let found = trace.as_ref().and_then(|(trace, path)| {
            Some((trace.action(path)?, Places::new(&trace.directory, &traces.limits)))
        });

        return Some((local(&variable, regex_field(kind)?, &pattern, &groups, found), range))
    }

    let value = match (kind, field) {
        (_, "flags") => flags(regex_field(kind)?, &literal.value),
        ("replacer", "to") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            replacements(trace.action(&path), &trace, &traces.limits)
        }
        ("replacer", "from") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            pattern(&literal.value, &groups(&outline, action), "located content(s)", Some((&trace, &path)), &language, &traces.limits)
        }
        ("builder", "match") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            pattern(&literal.value, &groups(&outline, action), "input(s)", Some((&trace, &path)), &language, &traces.limits)
        }
        ("builder", "build") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            built(trace.action(&path), &trace, &language, &traces.limits)
        }
        ("builder", "collect" | "separator") => {
            let collect = outline.field(action, "collect").is_some_and(|literal| literal.value == "true");
            let separator = outline.field(action, "separator").map(|literal| literal.value.clone()).unwrap_or_default();
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            let of = match outline.field(action, "reference-location") {
                Some(_) => "reference location inside the place of its location",
                None => "input of its location"
            };
            collected(collect.then_some(of), &separator, trace.action(&path), &language, &traces.limits)
        }
        ("maker", "prepare") => {
            let traced = traced(registry, project, traces, text, &outline, action);
            let traced = traced.as_ref().map(|(trace, path)| (trace.as_ref(), path.as_str()));
            pattern(&literal.value, &groups(&outline, action), ARGUMENTS, traced, &language, &traces.limits)
        }
        ("maker", "make") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            results(trace.action(&path), &trace, &language, &traces.limits)
        }
        ("list" | "switch", _) => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            let choices = choices(&outline, action, kind);
            let used = trace.action(&path);

            format!("{}\n\n{}", indexed(&choices, used, &trace), results(used, &trace, &language, &traces.limits))
        }
        _ => return None
    };

    Some((value, literal.range))
}

fn files(registry: &Registry, project: &Path, outline: &Outline) -> Option<String> {
    let (directory, files) = target::files(registry, project, outline)?;

    if files.is_empty() {
        return Some("Matches no target file".to_string())
    }

    Some(listing(format!("Matches {} target file(s)", files.len()), &directory, &files))
}

pub fn listing(title: String, directory: &Path, files: &[PathBuf]) -> String {
    let mut value = title + "\n";
    for file in files.iter().take(LISTED) {
        value += &match Url::from_file_path(directory.join(file)) {
            Ok(url) => format!("\n- [`{}`](<{url}>)", file.display()),
            Err(_) => format!("\n- `{}`", file.display())
        };
    }
    if files.len() > LISTED {
        value += &format!("\n- +{} more", files.len() - LISTED);
    }

    value
}

fn listed(registry: &Registry) -> String {
    registry.languages().iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
}

pub fn settings(registry: &Registry, project: &Path, text: &str, offset: usize) -> Option<(String, Range<usize>)> {
    let outline = Outline::parse(text);
    let node = outline.nodes.iter().enumerate()
        .find(|(index, node)| {
            let end = node.arguments.last().map(|literal| literal.range.end).unwrap_or(node.name_range.end);
            let parent = node.parent.unwrap_or(*index);

            outline.is_top(parent, "ast-set") && node.name_range.start <= offset && offset <= end
        })
        .map(|(_, node)| node)?;
    let end = node.arguments.last().map(|literal| literal.range.end).unwrap_or(node.name_range.end);

    let ast_set = from_kdl(text).ok()?.ast_set;
    let solution = target::operation(project).and_then(|operation| operation.target(registry).ok());
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for file in solution.iter().flat_map(|solution| solution.files()) {
        *counts.entry(ast_set.language(file).unwrap_or_default()).or_default() += 1;
    }

    let title = match &ast_set {
        AstSet::Language(name) => format!("Every file is `{name}`"),
        AstSet::Target => "The target names each file's language".to_string(),
        AstSet::Files(_) => "Files get the language of the last line they match".to_string()
    };
    let lines: Vec<String> = counts.into_iter()
        .map(|(name, count)| match name {
            "" => format!("- {count} file(s) without a language"),
            name => format!("- `{name}`: {count} file(s)")
        })
        .collect();

    Some((format!("{title}\n\n{}\n\nKnown languages are {}", lines.join("\n"), listed(registry)), node.name_range.start..end))
}

fn alias(registry: &Registry, name: &str) -> String {
    if registry.backend(name).is_none() {
        return format!("No language backend named `{name}` is registered, known languages are {}", listed(registry))
    }

    format!("Language backend `{name}`, used instead of the project's `ast-set`")
}

fn choices(outline: &Outline, action: usize, kind: &str) -> Vec<(String, String)> {
    let field = if kind == "list" { "values" } else { "cases" };
    let Some(node) = outline.nodes.iter().find(|node| node.parent == Some(action) && node.name == field) else { return vec![] };

    match kind {
        "list" => node.arguments.iter().enumerate().map(|(index, value)| (index.to_string(), value.value.clone())).collect(),
        _ => node.properties.iter().map(|(key, value)| (key.clone(), value.value.clone())).collect()
    }
}

fn indexed(choices: &[(String, String)], used: Option<&ActionTrace>, trace: &Trace) -> String {
    let lines: Vec<String> = choices.iter()
        .map(|(key, value)| {
            let mark = if used.is_some_and(|used| used.keys.contains(key)) { "✓" } else { "·" };
            format!("- {mark} `{key}` → {}", inline(value))
        })
        .collect();

    let unused = choices.iter().filter(|(key, _)| !used.is_some_and(|used| used.keys.contains(key))).count();
    let summary = match unused {
        0 => format!("Every choice is used across {} target file(s)", trace.files),
        _ => format!("{unused} of {} choice(s) never used across {} target file(s)", choices.len(), trace.files)
    };

    format!("{}\n\n{summary}", lines.join("\n"))
}

fn results(action: Option<&ActionTrace>, trace: &Trace, language: &str, limits: &Limits) -> String {
    let Some(action) = action.filter(|action| action.calls > 0) else {
        return format!("Not called in {} target file(s)", trace.files)
    };

    let mut value = format!("Called {} time(s) across {} target file(s)", action.calls, trace.files);
    for result in &action.results {
        let arguments: Vec<String> = result.groups.iter().flatten().map(|argument| inline(argument)).collect();
        let with = if arguments.is_empty() { "without arguments".to_string() } else { format!("with {}", arguments.join(", ")) };

        value += &format!("\n\n`{}`, {with}:\n{}", result.file.display(), fenced(language, &truncate(&result.output, limits)));
    }

    value
}

fn location(
    registry: &Registry, project: &Path, traces: &mut Traces, outline: &Outline, text: &str, literal: Literal, offset: usize
) -> Option<(String, Range<usize>)> {
    let backends = backends(registry, project, outline);
    let backend = *backends.first()?;
    let location = literal.value.as_str();
    let mut find = |locations: &[&str], title: &str| {
        let (directory, found) = traces.places(registry, project, &target::patch(outline), locations)?;
        let section = found.last().map(|located| placed(title, located, &directory, backend.name(), &traces.limits));

        Some((found.first()?.matches, section?))
    };

    if is_query(&literal) && literal.content.start <= offset && offset <= literal.content.end {
        let source = &text[literal.content.clone()];
        let found = part_at(source, offset - literal.content.start).filter(|(_, word)| !word.is_empty());

        if let Some((part, word)) = found {
            let range = literal.content.start + word.start..literal.content.start + word.end;
            let word = &source[word];
            let is_valid = backends.iter().any(|backend| backend.check_location(location).is_ok());
            let kind = (part == Part::Kind && is_kind(backend, word)).then(|| format!("({word}) @target"));

            let wanted: Vec<&str> = kind.as_deref().into_iter().chain(is_valid.then_some(location)).collect();
            let found = find(&wanted, "Places of this location");
            let mut value = query(backend, part, word);
            if let (Some(_), Some((matches, _))) = (&kind, &found) { value += &summary(*matches); }
            if let (true, Some((_, section))) = (is_valid, found) { value += &section; }

            return Some((value, range))
        }
    }

    let mut described = vec![];
    for backend in &backends {
        match backend.describe_location(location) {
            Ok(description) => described.push(description),
            Err(error) => return Some((format!("Not a {} location: {error}", backend.name()), literal.range))
        }
    }
    let section = find(&[location], "Places").map(|(_, section)| section);

    Some((described.join("\n\n") + &section.unwrap_or_default(), literal.range))
}

fn placed(title: &str, located: &LocationPlaces, directory: &Path, language: &str, limits: &Limits) -> String {
    let LocationMatches { files, matched_files, places: all } = located.matches;
    let mut value = format!("\n\n**{title}**\n\nIn {matched_files} of {files} target file(s), {all} in all");

    let mut places = Places::new(directory, limits);
    for Found { file, matched, .. } in &located.found {
        let place = places.link(file, matched.place.as_ref()).unwrap_or_else(|| format!("`{}`", file.display()));

        value += &format!("\n\n{}", snippet(&place, &matched.text, language, limits));
    }
    if all > located.found.len() {
        value += &format!("\n\n+{} more, Find All References lists every place", all - located.found.len());
    }

    value
}

fn summary(matches: LocationMatches) -> String {
    format!("\n\nMatches {} place(s) in {} of {} target file(s)", matches.places, matches.matched_files, matches.files)
}

fn is_kind(backend: &dyn LanguageBackend, word: &str) -> bool {
    word != "_" && backend.node_kinds().iter().any(|kind| kind == word)
}

fn query(backend: &dyn LanguageBackend, part: Part, word: &str) -> String {
    let name = backend.name();

    match part {
        Part::Kind if word == "_" => "Any named node".to_string(),
        Part::Kind if is_kind(backend, word) => format!("`{word}`: {name} node kind"),
        Part::Kind => format!("`{word}` is not a {name} node kind"),
        Part::Field if backend.fields().iter().any(|field| field == word.trim_start_matches('!')) => format!("`{word}`: {name} field"),
        Part::Field => format!("`{word}` is not a {name} field"),
        Part::Predicate => match PREDICATES.iter().find(|(predicate, _)| *predicate == word) {
            Some((_, meaning)) => format!("`#{word}`: {meaning}"),
            None => format!("`#{word}` does not filter matches")
        },
        Part::Capture if word == "target" => "`@target`: the place this location points at".to_string(),
        Part::Capture => format!("`@{word}`: a capture, for predicates to refer to")
    }
}

fn traced(
    registry: &Registry, project: &Path, traces: &mut Traces, text: &str, outline: &Outline, action: usize
) -> Option<(Arc<Trace>, String)> {
    let path = outline.action_path(action)?;

    Some((traces.get(registry, project, text)?, path))
}

fn local(
    variable: &str, field: &str, pattern: &str, groups: &[Group], found: Option<(&ActionTrace, Places)>
) -> String {
    let Some(number) = group_number(groups, variable) else {
        return format!("`$[{variable}]` has no capture group in `{field}`")
    };

    let mut value = format!(
        "`$[{variable}]` is group {} of `{field}`: {}", number + 1, inline(&pattern[groups[number].range.clone()])
    );

    let seen: Vec<String> = found.into_iter()
        .flat_map(|(action, mut places)| {
            action.found.iter()
                .filter_map(|found| Some(places.value(&found.file, found.groups.get(number)?.as_ref()?)))
                .collect::<Vec<_>>()
        })
        .collect();
    if !seen.is_empty() {
        value += &format!("\n\nSeen as {}", seen.join(", "));
    }

    value
}

fn argument(variable: &str, name: &str, calls: Option<&ActionTrace>) -> String {
    let Ok(number) = variable.parse::<usize>() else {
        return format!("`@[{variable}]` must be an argument number")
    };
    let mut value = format!("`@[{variable}]` is argument {} of `#[{name}]`", number + 1);

    let Some(calls) = calls.filter(|calls| calls.calls > 0) else { return value };
    value += &format!("\n\nPassed in {} of {} call(s)", calls.passed.get(number).unwrap_or(&0), calls.calls);

    let seen: Vec<String> = calls.results.iter()
        .filter_map(|result| result.groups.get(number)?.as_deref())
        .map(inline)
        .collect();
    if !seen.is_empty() {
        value += &format!("\n\nSeen as {}", seen.join(", "));
    }

    value
}

fn flags(field: &str, flags: &str) -> String {
    let mut lines: Vec<String> = FLAGS.iter()
        .map(|(flag, meaning)| {
            let mark = if flags.contains(*flag) { "✓" } else { "·" };
            format!("- {mark} `{flag}` {meaning}")
        })
        .collect();

    let unknown: String = flags.chars().filter(|flag| !FLAGS.iter().any(|(known, _)| known == flag)).collect();
    if !unknown.is_empty() {
        lines.push(format!("\nUnknown: {}", inline(&unknown)));
    }

    format!("Regex flags for `{field}`\n\n{}", lines.join("\n"))
}

fn replacements(action: Option<&ActionTrace>, trace: &Trace, limits: &Limits) -> String {
    let Some(action) = action.filter(|action| action.matches > 0) else {
        let inputs = action.map(|action| action.inputs).unwrap_or_default();
        return format!("No replacements: `from` matched nothing in {inputs} located content(s) across {} target file(s)", trace.files)
    };

    let mut value = format!("**{} replacement(s)** across {} target file(s)", action.matches, trace.files);
    let mut places = Places::new(&trace.directory, limits);
    for sample in &action.samples {
        value += &format!("\n\n{}\n{}", places.file(&sample.file, sample.place.as_ref()), diff(&sample.input, &sample.output, limits));
    }

    value
}

fn pattern(
    pattern: &str, groups: &[Group], inputs: &str, traced: Option<(&Trace, &str)>, language: &str, limits: &Limits
) -> String {
    let captures = match groups.is_empty() {
        true => "None".to_string(),
        false => groups.iter().enumerate()
            .map(|(index, group)| {
                let name = group.name.as_ref().map(|name| format!(", `$[{name}]`")).unwrap_or_default();
                format!("- `$[{index}]`{name} = {}", inline(&pattern[group.range.clone()]))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut value = format!("**Captures**\n\n{captures}");

    let Some((trace, action)) = traced.and_then(|(trace, path)| Some((trace, trace.action(path)?))) else { return value };
    value += &format!(
        "\n\n**Matches**\n\nIn {} of {} {inputs} across {} target file(s), {} in all",
        action.matched, action.inputs, trace.files, action.matches
    );

    let mut places = Places::new(&trace.directory, limits);
    for Found { file, matched, groups } in &action.found {
        let place = match inputs {
            ARGUMENTS => format!("`{}`", file.display()),
            _ => places.file(file, matched.place.as_ref())
        };

        value += &format!("\n\n{}", snippet(&place, &matched.text, language, limits));
        for (index, group) in groups.iter().enumerate() {
            let group = group.as_ref().map(|group| places.capture(file, group, language));
            value += &format!("\n- `$[{index}]`{}", group.unwrap_or_else(|| " = unset".to_string()));
        }
    }

    value
}

fn snippet(place: &str, text: &str, language: &str, limits: &Limits) -> String {
    match text {
        "" => format!("{place}: empty"),
        _ => format!("{place}\n{}", fenced(language, &truncate(text, limits)))
    }
}

fn built(action: Option<&ActionTrace>, trace: &Trace, language: &str, limits: &Limits) -> String {
    let Some(action) = action.filter(|action| action.inputs > 0) else {
        return format!("Never built in {} target file(s)", trace.files)
    };

    let mut value = format!("**Built from {} input(s)** across {} target file(s)", action.inputs, trace.files);
    let mut places = Places::new(&trace.directory, limits);
    for sample in &action.samples {
        value += &format!(
            "\n\n{}\n\nBuilds\n{}",
            snippet(&places.file(&sample.file, sample.place.as_ref()), &sample.input, language, limits),
            fenced(language, &truncate(&sample.output, limits))
        );
    }

    value
}

fn collected(
    collect: Option<&str>, separator: &str, action: Option<&ActionTrace>, language: &str, limits: &Limits
) -> String {
    let mut value = match collect {
        Some(of) => format!("Collects every {of}, joined by {}", inline(separator)),
        None => "Builds from the content around each call only".to_string()
    };

    if let Some(result) = action.and_then(|action| action.results.first()) {
        value += &format!("\n\nFirst result in `{}`:\n{}", result.file.display(), fenced(language, &truncate(&result.output, limits)));
    }

    value
}
