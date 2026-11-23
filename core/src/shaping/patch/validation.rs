// Standard Uses
use std::collections::{HashMap, HashSet};
use std::ops::Range;

// Crate Uses
use crate::ast::backend::{LanguageBackend, LocationError};
use crate::expressions::syntax::ExpressionAST;
use crate::expressions::syntax::parser::parse_expression;
use crate::shaping::patch::{pattern, Actions, Patch};
use crate::shaping::patch::replacer::{check_flags, regex_builder};
use crate::shaping::patch::source::{ActionSpans, NodeSpans, PatchSource};

// External Uses
use miette::SourceSpan;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    IncorrectRegex,
    LocationNotFound,
    ActionNotFound,
    IncorrectArgs,
    IncorrectExpression,
    DuplicateAction,
    RecursiveAction,
    UnusedAction,
    IncorrectFilePattern,
    FileNotMatched
}

impl DiagnosticKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::IncorrectRegex => "incorrect-regex",
            Self::LocationNotFound => "location-not-found",
            Self::ActionNotFound => "action-not-found",
            Self::IncorrectArgs => "incorrect-args",
            Self::IncorrectExpression => "incorrect-expression",
            Self::DuplicateAction => "duplicate-action",
            Self::RecursiveAction => "recursive-action",
            Self::UnusedAction => "unused-action",
            Self::IncorrectFilePattern => "incorrect-file-pattern",
            Self::FileNotMatched => "file-not-matched"
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub action: String,
    pub field: &'static str,
    pub span: Option<SourceSpan>
}

impl Diagnostic {
    pub fn new(kind: DiagnosticKind, action: &str, field: &'static str, message: String) -> Self {
        let severity = match kind {
            DiagnosticKind::UnusedAction | DiagnosticKind::FileNotMatched => Severity::Warning,
            _ => Severity::Error
        };

        Self { kind, severity, message, action: action.to_string(), field, span: None }
    }

    fn at(mut self, span: Option<SourceSpan>) -> Self {
        self.span = span;
        self
    }
}


pub fn validate(patch: &Patch, backend: Option<&dyn LanguageBackend>) -> Vec<Diagnostic> {
    check(patch, None, backend)
}

pub fn validate_source(patch: &Patch, source: &PatchSource, backend: Option<&dyn LanguageBackend>) -> Vec<Diagnostic> {
    check(patch, Some(source), backend)
}

fn check(patch: &Patch, source: Option<&PatchSource>, backend: Option<&dyn LanguageBackend>) -> Vec<Diagnostic> {
    let mut validator = Validator { backend, source, ..Validator::default() };
    validator.actions(&patch.actions, "", source.map(|source| &source.spans.actions));

    let calls = std::mem::take(&mut validator.calls);
    for (action, field, name, count, span) in &calls {
        validator.current_span = *span;

        if !validator.names.contains_key(name) {
            validator.report(DiagnosticKind::ActionNotFound, action, field, format!("No action named '{name}'"));
            continue
        }

        if leads_to(&calls, &validator.names, name, action) {
            validator.report(DiagnosticKind::RecursiveAction, action, field, format!("'#[{name}]' calls back to this action"));
        }

        let required = validator.required.get(name).copied().unwrap_or(0);

        if !validator.receivers.contains(name) && *count > 0 {
            let message = format!("'#[{name}]' takes no arguments, but is called with {count}");
            validator.report(DiagnosticKind::IncorrectArgs, action, field, message);
        } else if *count < required {
            let message = format!(
                "'#[{name}]' is called with {count} argument(s), but uses '@[{}]'", required - 1
            );
            validator.report(DiagnosticKind::IncorrectArgs, action, field, message);
        }
    }

    let called: HashSet<&String> = calls.iter().map(|(_, _, name, _, _)| name).collect();
    let mut unused: Vec<(&String, &(String, Option<SourceSpan>))> = validator.names.iter()
        .filter(|(name, _)| !called.contains(name))
        .collect();
    unused.sort_by_key(|(_, (path, _))| path.to_string());

    let mut diagnostics = validator.diagnostics.clone();
    for (name, (path, span)) in unused {
        diagnostics.push(Diagnostic::new(
            DiagnosticKind::UnusedAction, path, "name", format!("Action '{name}' is never used")
        ).at(*span));
    }

    diagnostics
}



fn leads_to(
    calls: &[Call], names: &HashMap<String, (String, Option<SourceSpan>)>, name: &str, target: &str
) -> bool {
    let mut pending = vec![name];
    let mut seen = HashSet::new();

    while let Some(name) = pending.pop() {
        if !seen.insert(name) { continue }
        let Some((path, _)) = names.get(name) else { continue };
        if path == target { return true }

        pending.extend(calls.iter().filter(|call| call.0 == *path).map(|call| call.2.as_str()));
    }

    false
}

fn join(parent: &str, action: &str) -> String {
    if parent.is_empty() { action.to_string() } else { format!("{parent}/{action}") }
}


enum Locals {
    Groups { count: usize, names: Vec<String> },
    Unavailable(&'static str),
    Unknown
}

type Call = (String, &'static str, String, usize, Option<SourceSpan>);

#[derive(Default)]
struct Validator<'a> {
    backend: Option<&'a dyn LanguageBackend>,
    source: Option<&'a PatchSource>,
    diagnostics: Vec<Diagnostic>,
    names: HashMap<String, (String, Option<SourceSpan>)>,
    receivers: HashSet<String>,
    required: HashMap<String, usize>,
    calls: Vec<Call>,
    current: Option<&'a NodeSpans>,
    current_span: Option<SourceSpan>
}

impl<'a> Validator<'a> {
    fn report(&mut self, kind: DiagnosticKind, action: &str, field: &'static str, message: String) {
        let span = self.span(field);
        self.diagnostics.push(Diagnostic::new(kind, action, field, message).at(span));
    }

    fn report_at(
        &mut self, kind: DiagnosticKind, action: &str, field: &'static str, message: String, range: Option<Range<usize>>
    ) {
        let span = self.narrow(field, range);
        self.diagnostics.push(Diagnostic::new(kind, action, field, message).at(span));
    }

    fn narrow(&self, field: &str, range: Option<Range<usize>>) -> Option<SourceSpan> {
        let span = self.span(field)?;
        let narrowed = self.source.zip(range).and_then(|(source, range)| source.narrow(span, range));

        Some(narrowed.unwrap_or(span))
    }

    fn span(&self, field: &str) -> Option<SourceSpan> {
        self.current_span.or_else(|| self.current?.fields.get(field).copied())
    }

    fn enter(&mut self, spans: Option<&'a [NodeSpans]>, index: usize) {
        self.current = spans.and_then(|spans| spans.get(index));
        self.current_span = None;
    }

    fn actions(&mut self, actions: &Actions, parent: &str, spans: Option<&'a ActionSpans>) {
        for (index, builder) in actions.builders.iter().flatten().enumerate() {
            self.enter(spans.map(|spans| spans.builders.as_slice()), index);
            let nested = self.current.map(|current| &current.actions);
            let path = self.name(parent, &builder.name);

            self.location(&path, "location", &builder.location);
            if let Some(reference_location) = &builder.reference_location {
                self.location(&path, "reference-location", reference_location);
            }

            let locals = self.regex(&path, "match", &builder.r#match, "");
            self.expression(&path, "build", &builder.build, &locals, None);
            self.actions(&builder.actions, &path, nested);
        }

        for (index, replacer) in actions.replacers.iter().flatten().enumerate() {
            self.enter(spans.map(|spans| spans.replacers.as_slice()), index);
            let nested = self.current.map(|current| &current.actions);
            let path = join(parent, &format!("replacer #{}", index + 1));

            self.location(&path, "location", &replacer.location);
            if !replacer.reference_location.is_empty() {
                self.location(&path, "reference-location", &replacer.reference_location);
            }

            if let Err(error) = check_flags(&replacer.flags) {
                self.report(DiagnosticKind::IncorrectRegex, &path, "flags", error.to_string());
            }

            let locals = self.regex(&path, "from", &replacer.from, &replacer.flags);
            self.expression(&path, "to", &replacer.to, &locals, None);
            self.actions(&replacer.actions, &path, nested);
        }

        for (index, maker) in actions.makers.iter().flatten().enumerate() {
            self.enter(spans.map(|spans| spans.makers.as_slice()), index);
            let path = self.name(parent, &maker.name);
            self.receivers.insert(maker.name.clone());

            let locals = match &maker.prepare {
                Some(prepare) => self.regex(&path, "prepare", prepare, ""),
                None => Locals::Unavailable("the maker has no 'prepare' regex")
            };
            self.expression(&path, "make", &maker.make, &locals, Some(&maker.name));
        }

        for (index, list) in actions.lists.iter().flatten().enumerate() {
            self.enter(spans.map(|spans| spans.lists.as_slice()), index);
            let path = self.name(parent, &list.name);
            self.receivers.insert(list.name.clone());
            if let Some(index) = &list.index {
                self.expression(&path, "index", index, &Locals::Unavailable("'index' has no regex"), Some(&list.name));
            }
        }

        for (index, switch) in actions.switchers.iter().flatten().enumerate() {
            self.enter(spans.map(|spans| spans.switchers.as_slice()), index);
            let path = self.name(parent, &switch.name);
            self.receivers.insert(switch.name.clone());
            self.expression(&path, "index", &switch.index, &Locals::Unavailable("'index' has no regex"), Some(&switch.name));
        }
    }

    fn name(&mut self, parent: &str, name: &str) -> String {
        let path = join(parent, name);

        if let Some((first, _)) = self.names.get(name) {
            let message = format!("Action '{name}' is already defined at '{first}', so this one is never used");
            self.report(DiagnosticKind::DuplicateAction, &path, "name", message);
        } else {
            let span = self.span("name");
            self.names.insert(name.to_string(), (path.clone(), span));
        }

        path
    }

    fn location(&mut self, action: &str, field: &'static str, location: &str) {
        let Some(backend) = self.backend else { return };

        if let Err(error) = backend.check_location(location) {
            let range = error.downcast_ref::<LocationError>().and_then(|error| error.range.clone());
            self.report_at(DiagnosticKind::LocationNotFound, action, field, error.to_string(), range);
        }
    }

    fn regex(&mut self, action: &str, field: &'static str, regex: &str, flags: &str) -> Locals {
        match regex_builder(regex, flags).build() {
            Ok(regex) => Locals::Groups {
                count: regex.captures_len() - 1,
                names: regex.capture_names().flatten().map(str::to_string).collect()
            },
            Err(error) => {
                let (message, range) = match pattern::error(regex, flags) {
                    Some((message, range)) => (message, Some(range)),
                    None => (error.to_string(), None)
                };
                self.report_at(DiagnosticKind::IncorrectRegex, action, field, message, range);
                Locals::Unknown
            }
        }
    }

    fn expression(&mut self, action: &str, field: &'static str, text: &str, locals: &Locals, arguments: Option<&str>) {
        match parse_expression(text) {
            Ok(expression) => self.walk(action, field, &expression, locals, arguments),
            Err(error) => self.report_at(DiagnosticKind::IncorrectExpression, action, field, error.message, Some(error.span))
        }
    }

    fn walk(
        &mut self, action: &str, field: &'static str, expression: &ExpressionAST, locals: &Locals, arguments: Option<&str>
    ) {
        match expression {
            ExpressionAST::Action(parts) => for part in parts {
                self.walk(action, field, part, locals, arguments);
            },
            ExpressionAST::ActionCall { name, arguments: call_arguments, span } => {
                let span = self.narrow(field, Some(span.clone()));
                self.calls.push((action.to_string(), field, name.to_string(), call_arguments.len(), span));

                for argument in call_arguments {
                    self.walk(action, field, argument, locals, arguments);
                }
            }
            ExpressionAST::LocalCall { name, span } => self.local(action, field, name, span, locals),
            ExpressionAST::ArgumentCall { name: variable, span } => match (arguments, variable.parse::<usize>()) {
                (None, _) => {
                    let message = format!("'@[{variable}]' is used where no arguments exist");
                    self.report_at(DiagnosticKind::IncorrectArgs, action, field, message, Some(span.clone()));
                }
                (Some(_), Err(_)) => {
                    let message = format!("'@[{variable}]' must be an argument number");
                    self.report_at(DiagnosticKind::IncorrectArgs, action, field, message, Some(span.clone()));
                }
                (Some(receiver), Ok(index)) => {
                    let required = self.required.entry(receiver.to_string()).or_default();
                    *required = (*required).max(index + 1);
                }
            },
            ExpressionAST::Text(_) => {}
        }
    }

    fn local(&mut self, action: &str, field: &'static str, variable: &str, span: &Range<usize>, locals: &Locals) {
        let message = match (locals, variable.parse::<usize>()) {
            (Locals::Unknown, _) => return,
            (Locals::Unavailable(reason), _) => format!("'$[{variable}]' has no capture group to refer to: {reason}"),
            (Locals::Groups { names, .. }, Err(_)) if names.iter().any(|name| name == variable) => return,
            (Locals::Groups { .. }, Err(_)) => format!("'$[{variable}]' is not a capture group number or name of the regex"),
            (Locals::Groups { count, .. }, Ok(index)) if index >= *count =>
                format!("'$[{index}]' needs {} capture group(s), but the regex has {count}", index + 1),
            (Locals::Groups { .. }, Ok(_)) => return
        };

        self.report_at(DiagnosticKind::IncorrectArgs, action, field, message, Some(span.clone()));
    }
}
