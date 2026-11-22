// Standard Uses
use std::path::Path;

// Crate Uses
use crate::document::{ast_set, kind, DocumentKind};
use crate::position::LineIndex;

// External Uses
use codeshaper::core::registry::Registry;
use codeshaper::core::shaping::operation::configuration;
use codeshaper::core::shaping::operation::patch_backend;
use codeshaper::core::shaping::patch::Patch;
use codeshaper::core::shaping::patch::validation::{validate_source, Severity};
use codeshaper::core::shaping::project::settings;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString};
use miette::Report;


const SOURCE: &str = "codeshaper";


pub fn diagnostics(registry: &Registry, path: &Path, text: &str) -> Vec<Diagnostic> {
    let index = LineIndex::new(text);

    let result = match kind(path) {
        DocumentKind::Patch { project } => return patch(registry, &project, path, text, &index),
        DocumentKind::Settings => settings::from_kdl(text).map(|_| ()),
        DocumentKind::Configuration => configuration::from_kdl(text).map(|_| ()),
        DocumentKind::TargetProject => codeshaper_target_text::project::from_kdl(text).map(|_| ()),
        DocumentKind::Other => Ok(())
    };

    result.err().map(|report| parse_errors(&report, &index)).unwrap_or_default()
}

fn patch(registry: &Registry, project: &Path, path: &Path, text: &str, index: &LineIndex) -> Vec<Diagnostic> {
    let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();

    let (patch, source) = match Patch::parse(&name, text) {
        Ok(parsed) => parsed,
        Err(report) => return parse_errors(&report, index)
    };

    let backend = patch_backend(registry, &patch, &ast_set(project));

    validate_source(&patch, &source, backend).into_iter()
        .map(|diagnostic| {
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
        })
        .collect()
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
