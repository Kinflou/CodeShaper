// Standard Uses
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Crate Uses
use crate::groups::{arguments, group_number, groups, locals};
use crate::navigation::{calls, symbol_at, Symbol};
use crate::query::{is_query, part_at, Part, PREDICATES};
use crate::schema::{regex_field, LOCATIONS};
use crate::section::{view_at, View};
use crate::syntax::{Literal, Outline, ACTIONS};
use crate::target::{self, backends, matches};
use crate::trace::{diff, fenced, inline, truncate, Limits, Traces};

// External Uses
use codeshaper::core::ast::backend::LanguageBackend;
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::LocationMatches;
use codeshaper::core::shaping::patch::pattern::{Group, FLAGS};
use codeshaper::core::shaping::patch::trace::{ActionTrace, Trace};
use codeshaper::core::shaping::project::ast_set::AstSet;
use codeshaper::core::shaping::project::settings::from_kdl;
use url::Url;


const LISTED: usize = 10;


pub fn hover(
    registry: &Registry, project: &Path, traces: &mut Traces, text: &str, offset: usize
) -> Option<(String, Range<usize>)> {
    let View { text, .. } = view_at(text, offset);
    let text = text.as_ref();
    let outline = Outline::parse(text);
    let language = backends(registry, project, &outline).first().map(|backend| backend.name().to_string()).unwrap_or_default();

    if let Some(Symbol { name, range }) = symbol_at(&outline, text, offset) {
        let index = outline.nodes.iter().position(|node| {
            ACTIONS.contains(&node.name.as_str()) && node.arguments.first().is_some_and(|literal| literal.value == name)
        })?;

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
        return location(registry, project, &outline, text, literal, offset)
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
    if let Some((variable, range)) = passed.filter(|_| matches!((kind, field), ("maker", "make") | ("list" | "switch", "index"))) {
        let traced = traced(registry, project, traces, text, &outline, action);
        let calls = traced.as_ref().and_then(|(trace, path)| trace.action(path));

        return Some((argument(&variable, outline.nodes[action].arguments.first()?.value.as_str(), calls), range))
    }

    let used = locals(text, &literal).into_iter().find(|(_, range)| range.start <= offset && offset <= range.end);
    if let Some((variable, range)) = used {
        let pattern = outline.field(action, regex_field(kind)?).map(|literal| literal.value.clone()).unwrap_or_default();
        let groups = groups(&outline, action);
        let trace = traced(registry, project, traces, text, &outline, action);
        let samples = trace.as_ref().and_then(|(trace, path)| trace.action(path));

        return Some((local(&variable, regex_field(kind)?, &pattern, &groups, samples), range))
    }

    let value = match (kind, field) {
        (_, "flags") => flags(regex_field(kind)?, &literal.value),
        ("replacer", "to") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            replacements(trace.action(&path), &trace, &traces.limits)
        }
        ("replacer", "from") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            pattern(&literal.value, &groups(&outline, action), "located content(s)", trace.action(&path), trace.files)
        }
        ("builder", "match") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            pattern(&literal.value, &groups(&outline, action), "input(s)", trace.action(&path), trace.files)
        }
        ("builder", "build") => {
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            built(trace.action(&path), &trace, &language, &traces.limits)
        }
        ("builder", "collect" | "separator") => {
            let collect = outline.field(action, "collect").is_some_and(|literal| literal.value == "true");
            let separator = outline.field(action, "separator").map(|literal| literal.value.clone()).unwrap_or_default();
            let (trace, path) = traced(registry, project, traces, text, &outline, action)?;
            collected(collect, &separator, trace.action(&path), &language, &traces.limits)
        }
        ("maker", "prepare") => {
            let traced = traced(registry, project, traces, text, &outline, action);
            let (used, files) = traced.as_ref().map(|(trace, path)| (trace.action(path), trace.files)).unwrap_or_default();
            pattern(&literal.value, &groups(&outline, action), "first argument(s)", used, files)
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
    registry: &Registry, project: &Path, outline: &Outline, text: &str, literal: Literal, offset: usize
) -> Option<(String, Range<usize>)> {
    let backends = backends(registry, project, outline);
    let backend = *backends.first()?;

    if is_query(&literal) && literal.content.start <= offset && offset <= literal.content.end {
        let source = &text[literal.content.clone()];
        let found = part_at(source, offset - literal.content.start).filter(|(_, word)| !word.is_empty());

        if let Some((part, word)) = found {
            let range = literal.content.start + word.start..literal.content.start + word.end;
            let counted = |query: &str| matches(registry, project, outline, query).map(summary).unwrap_or_default();

            return Some((query(backend, part, &source[word], source, counted), range))
        }
    }

    let mut described = vec![];
    for backend in &backends {
        match backend.describe_location(&literal.value) {
            Ok(description) => described.push(description),
            Err(error) => return Some((format!("Not a {} location: {error}", backend.name()), literal.range))
        }
    }
    let description = described.join("\n\n");

    let found = matches(registry, project, outline, &literal.value).map(summary);

    Some((description + &found.unwrap_or_default(), literal.range))
}

fn summary(matches: LocationMatches) -> String {
    format!("\n\nMatches {} place(s) in {} of {} target file(s)", matches.places, matches.matched_files, matches.files)
}

fn query(backend: &dyn LanguageBackend, part: Part, word: &str, source: &str, counted: impl Fn(&str) -> String) -> String {
    let name = backend.name();

    match part {
        Part::Kind if word == "_" => "Any named node".to_string(),
        Part::Kind if backend.node_kinds().iter().any(|kind| kind == word) =>
            format!("`{word}`: {name} node kind{}", counted(&format!("({word}) @target"))),
        Part::Kind => format!("`{word}` is not a {name} node kind"),
        Part::Field if backend.fields().iter().any(|field| field == word.trim_start_matches('!')) => format!("`{word}`: {name} field"),
        Part::Field => format!("`{word}` is not a {name} field"),
        Part::Predicate => match PREDICATES.iter().find(|(predicate, _)| *predicate == word) {
            Some((_, meaning)) => format!("`#{word}`: {meaning}"),
            None => format!("`#{word}` does not filter matches")
        },
        Part::Capture if word == "target" => format!("`@target`: the place this location points at{}", counted(source)),
        Part::Capture => format!("`@{word}`: a capture, for predicates to refer to")
    }
}

fn traced(
    registry: &Registry, project: &Path, traces: &mut Traces, text: &str, outline: &Outline, action: usize
) -> Option<(Arc<Trace>, String)> {
    let path = outline.action_path(action)?;

    Some((traces.get(registry, project, text)?, path))
}

fn local(variable: &str, field: &str, pattern: &str, groups: &[Group], samples: Option<&ActionTrace>) -> String {
    let Some(number) = group_number(groups, variable) else {
        return format!("`$[{variable}]` has no capture group in `{field}`")
    };

    let mut value = format!(
        "`$[{variable}]` is group {} of `{field}`: {}", number + 1, inline(&pattern[groups[number].range.clone()])
    );

    let seen: Vec<String> = samples.into_iter()
        .flat_map(|samples| &samples.samples)
        .filter_map(|sample| sample.groups.get(number).cloned().flatten())
        .map(|value| inline(&value))
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
    for sample in &action.samples {
        value += &format!("\n\n`{}`\n{}", sample.file.display(), diff(&sample.input, &sample.output, limits));
    }

    value
}

fn pattern(pattern: &str, groups: &[Group], inputs: &str, action: Option<&ActionTrace>, files: usize) -> String {
    let mut value = match groups.is_empty() {
        true => "No capture groups".to_string(),
        false => groups.iter().enumerate()
            .map(|(index, group)| {
                let name = group.name.as_ref().map(|name| format!(", `$[{name}]`")).unwrap_or_default();
                format!("- `$[{index}]`{name} = {}", inline(&pattern[group.range.clone()]))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let Some(action) = action else { return value };
    value += &format!(
        "\n\nMatches in {} of {} {inputs} across {} target file(s), {} match(es) in all",
        action.matched, action.inputs, files, action.matches
    );

    if let Some(sample) = action.samples.iter().find(|sample| sample.matched) {
        value += &format!("\n\nFirst match in `{}`: {}", sample.file.display(), inline(&sample.input));
        for (index, group) in sample.groups.iter().enumerate() {
            value += &format!("\n- `$[{index}]` = {}", group.as_deref().map(inline).unwrap_or_else(|| "unset".to_string()));
        }
    }

    value
}

fn built(action: Option<&ActionTrace>, trace: &Trace, language: &str, limits: &Limits) -> String {
    let Some(action) = action.filter(|action| action.inputs > 0) else {
        return format!("Never built in {} target file(s)", trace.files)
    };

    let mut value = format!("**Built from {} input(s)** across {} target file(s)", action.inputs, trace.files);
    for sample in &action.samples {
        value += &format!(
            "\n\n`{}`: {}\n{}", sample.file.display(), inline(&truncate(&sample.input, limits)),
            fenced(language, &truncate(&sample.output, limits))
        );
    }

    value
}

fn collected(
    collect: bool, separator: &str, action: Option<&ActionTrace>, language: &str, limits: &Limits
) -> String {
    let mut value = match collect {
        true => format!("Collects every input of its location, joined by {}", inline(separator)),
        false => "Builds from the content around each call only".to_string()
    };

    if let Some(result) = action.and_then(|action| action.results.first()) {
        value += &format!("\n\nFirst result in `{}`:\n{}", result.file.display(), fenced(language, &truncate(&result.output, limits)));
    }

    value
}
