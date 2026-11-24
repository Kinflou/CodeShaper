// Standard Uses
use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;

// Crate Uses
use crate::document::{ast_set, kind, DocumentKind};
use crate::groups::links;
use crate::navigation::calls;
use crate::options::unknown;
use crate::paths::check;
use crate::position::LineIndex;
use crate::schema::regex_field;
use crate::syntax::{Outline, ACTION_NODES};
use crate::target::{backends, operation};

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::patch::{pattern, Patch};
use codeshaper::core::shaping::patch::trace::Trace;
use codeshaper::core::shaping::patch::validation::{validate_source, Diagnostic as PatchDiagnostic, Severity};
use codeshaper::core::shaping::project::settings::{self, ProjectSettings};
use lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, NumberOrString};
use miette::Report;


pub const SOURCE: &str = "codeshaper";


pub fn diagnostics(registry: &Registry, path: &Path, text: &str) -> Vec<Diagnostic> {
    let index = LineIndex::new(text);

    let result = match kind(path) {
        DocumentKind::Patch { project } => return patch(registry, &project, path, text, &index),
        DocumentKind::Settings => match settings::from_kdl(text) {
            Ok(settings) => {
                let mut found = unknown_languages(registry, &settings, text, &index);
                found.extend(listing(registry, path, text, &index));

                return found
            }
            Err(report) => Err(report)
        },
        DocumentKind::Configuration => configuration::from_kdl(text).map(|_| ()),
        DocumentKind::Other => return vec![]
    };

    match result {
        Ok(()) => listing(registry, path, text, &index),
        Err(report) => parse_errors(&report, &index)
    }
}

fn listing(registry: &Registry, path: &Path, text: &str, index: &LineIndex) -> Vec<Diagnostic> {
    let mut found: Vec<Diagnostic> = unknown(registry, path, text).into_iter()
        .map(|(range, code, message)| problem(index, &range, DiagnosticSeverity::ERROR, code, message))
        .collect();
    found.extend(check(path, text));

    found
}

pub fn problem(index: &LineIndex, range: &Range<usize>, severity: DiagnosticSeverity, code: &str, message: String) -> Diagnostic {
    Diagnostic {
        range: index.range(range.clone()),
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some(SOURCE.to_string()),
        message,
        ..Diagnostic::default()
    }
}

fn patch(registry: &Registry, project: &Path, path: &Path, text: &str, index: &LineIndex) -> Vec<Diagnostic> {
    let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();

    let (patch, source) = match Patch::parse(&name, text) {
        Ok(parsed) => parsed,
        Err(report) => return parse_errors(&report, index)
    };

    let backends = backends(registry, project, &Outline::parse(text));

    let mut found: Vec<Diagnostic> = validate_source(&patch, &source, backends).into_iter()
        .map(|diagnostic| convert(diagnostic, index))
        .collect();
    found.extend(regex_hints(text, index));
    found.extend(redundant_alias(project, text).map(|alias| Diagnostic {
        range: index.range(alias),
        severity: Some(DiagnosticSeverity::HINT),
        code: Some(NumberOrString::String("redundant-alias".to_string())),
        source: Some(SOURCE.to_string()),
        message: "alias: the project's 'ast-set' already names this language".to_string(),
        tags: Some(vec![DiagnosticTag::UNNECESSARY]),
        ..Diagnostic::default()
    }));

    found
}

pub fn redundant_alias(project: &Path, text: &str) -> Option<Range<usize>> {
    let outline = Outline::parse(text);
    let node = outline.top("alias")?;
    let alias = node.arguments.first()?;
    let ast_set = ast_set(project)?;

    (ast_set.fixed() == Some(alias.value.as_str())).then_some(node.name_range.start..alias.range.end)
}

fn unknown_languages(registry: &Registry, settings: &ProjectSettings, text: &str, index: &LineIndex) -> Vec<Diagnostic> {
    let outline = Outline::parse(text);
    let Some(position) = outline.nodes.iter().position(|node| node.parent.is_none() && node.name == "ast-set") else { return vec![] };
    let arguments = outline.nodes[position].arguments.iter().map(|literal| (literal.value.as_str(), literal.range.clone()));
    let lines = outline.nodes.iter().filter(|node| node.parent == Some(position)).map(|node| (node.name.as_str(), node.name_range.clone()));
    let names = settings.ast_set.names();

    arguments.chain(lines)
        .filter(|(name, _)| names.contains(name) && registry.backend(name).is_none())
        .map(|(name, range)| Diagnostic {
            range: index.range(range),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("language-not-found".to_string())),
            source: Some(SOURCE.to_string()),
            message: format!("ast-set: '{name}' is not a known language, known languages are: {}", registry.languages().join(", ")),
            ..Diagnostic::default()
        })
        .collect()
}

fn regex_hints(text: &str, index: &LineIndex) -> Vec<Diagnostic> {
    let outline = Outline::parse(text);
    let mut found = vec![];

    for (action, node) in outline.nodes.iter().enumerate() {
        let Some(field) = regex_field(&node.name) else { continue };
        let Some(regex) = outline.field(action, field) else { continue };
        let Some(path) = outline.action_path(action) else { continue };
        let links = links(&outline, text, action).unwrap_or_default();

        for (number, link) in links.iter().enumerate().filter(|(_, link)| link.uses.is_empty()) {
            found.push(Diagnostic {
                range: index.range(link.group.clone()),
                severity: Some(DiagnosticSeverity::HINT),
                code: Some(NumberOrString::String("unused-group".to_string())),
                source: Some(SOURCE.to_string()),
                message: format!("{path} › {field}: '$[{number}]' is never used, '(?:…)' groups without capturing"),
                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                ..Diagnostic::default()
            });
        }

        let flags = outline.field(action, "flags").map(|flags| flags.value.as_str()).unwrap_or_default();
        let placeholder = regex.value.is_empty() && node.name == "builder";

        if !placeholder && pattern::matches_empty(&regex.value, flags) {
            found.push(Diagnostic {
                range: index.range(regex.range.clone()),
                severity: Some(DiagnosticSeverity::HINT),
                code: Some(NumberOrString::String("matches-empty".to_string())),
                source: Some(SOURCE.to_string()),
                message: format!("{path} › {field}: can match empty text"),
                ..Diagnostic::default()
            });
        }
    }

    found
}

fn convert(diagnostic: PatchDiagnostic, index: &LineIndex) -> Diagnostic {
    let range = diagnostic.span
        .map(|span| index.range(span.offset()..span.offset() + span.len()))
        .unwrap_or_default();
    let severity = match diagnostic.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING
    };
    let message = format!("{} › {}: {}", diagnostic.action, diagnostic.field, diagnostic.message);

    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(diagnostic.kind.code().to_string())),
        source: Some(SOURCE.to_string()),
        message,
        ..Diagnostic::default()
    }
}

pub fn file(registry: &Registry, project: &Path, text: &str) -> Vec<Diagnostic> {
    let Ok((patch, source)) = Patch::parse("", text) else { return vec![] };
    let found = operation(project).and_then(|operation| operation.check_file(registry, &patch, &source).ok());
    let index = LineIndex::new(text);

    found.into_iter().flatten().map(|diagnostic| convert(diagnostic, &index)).collect()
}

pub fn coverage(text: &str, trace: &Trace) -> Vec<Diagnostic> {
    if trace.files == 0 { return vec![] }

    let outline = Outline::parse(text);
    let index = LineIndex::new(text);
    let called: HashSet<String> = calls(&outline, text).into_iter().map(|call| call.name).collect();
    let files = trace.files;
    let mut found = vec![];

    let mut report = |range: Range<usize>, severity, code: &str, message: String| found.push(Diagnostic {
        range: index.range(range),
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some(SOURCE.to_string()),
        message,
        ..Diagnostic::default()
    });

    for (position, node) in outline.nodes.iter().enumerate() {
        if !ACTION_NODES.contains(&node.name.as_str()) { continue }
        let Some(path) = outline.action_path(position) else { continue };
        let action = trace.action(&path).cloned().unwrap_or_default();
        let location = outline.field(position, "location").map(|literal| literal.range.clone());

        if node.name == "replacer" {
            match (action.inputs, location, outline.field(position, "from")) {
                (0, Some(location), _) => report(
                    location, DiagnosticSeverity::WARNING, "location-unmatched",
                    format!("{path} › location: matches nothing in {files} target file(s)")
                ),
                (inputs, _, Some(from)) if inputs > 0 && action.matches == 0 => report(
                    from.range.clone(), DiagnosticSeverity::WARNING, "from-unmatched",
                    format!("{path} › from: never matches in the {inputs} located content(s)")
                ),
                _ => {}
            }
            continue
        }

        let Some(name) = node.arguments.first() else { continue };
        if !called.contains(&name.value) { continue }

        if let (true, 0, Some(location)) = (node.name == "builder", action.inputs, location) {
            report(
                location, DiagnosticSeverity::WARNING, "location-unmatched",
                format!("{path} › location: matches nothing in {files} target file(s)")
            );
            continue
        }

        if action.calls == 0 {
            report(
                name.range.clone(), DiagnosticSeverity::HINT, "never-called",
                format!("{path}: never called in {files} target file(s)")
            );
            continue
        }

        if node.name != "switch" { continue }
        let cases = outline.nodes.iter().find(|child| child.parent == Some(position) && child.name == "cases");

        for (key, value) in cases.into_iter().flat_map(|cases| &cases.properties) {
            if action.keys.contains(key) { continue }
            report(
                value.range.clone(), DiagnosticSeverity::HINT, "case-unused",
                format!("{path} › cases: '{key}' is never used in {files} target file(s)")
            );
        }
    }

    found
}

fn parse_errors(report: &Report, index: &LineIndex) -> Vec<Diagnostic> {
    let error = |range, message: String| Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some(SOURCE.to_string()),
        message,
        ..Diagnostic::default()
    };

    let mut found = vec![];

    for related in report.related().into_iter().flatten() {
        let labels: Vec<_> = related.labels().into_iter().flatten().collect();
        if labels.is_empty() {
            found.push(error(Default::default(), related.to_string()));
        }

        for label in labels {
            let range = index.range(label.offset()..label.offset() + label.len());
            let message = match label.label() {
                Some(text) if !text.is_empty() => format!("{related}: {text}"),
                _ => related.to_string()
            };
            found.push(error(range, message));
        }
    }

    if found.is_empty() {
        found.push(error(Default::default(), report.to_string()));
    }

    found
}
