// Standard Uses
use std::collections::{HashMap, HashSet};

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::expressions::syntax::ExpressionAST;
use crate::expressions::syntax::parser::parse_expression;
use crate::shaping::patch::{Actions, Patch};

// External Uses
use regex::Regex;


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
    UnusedAction,
    IncorrectFilePattern,
    FileNotMatched
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub action: String,
    pub field: &'static str
}

impl Diagnostic {
    pub fn new(kind: DiagnosticKind, action: &str, field: &'static str, message: String) -> Self {
        let severity = match kind {
            DiagnosticKind::UnusedAction | DiagnosticKind::FileNotMatched => Severity::Warning,
            _ => Severity::Error
        };

        Self { kind, severity, message, action: action.to_string(), field }
    }
}


pub fn validate(patch: &Patch, backend: Option<&dyn LanguageBackend>) -> Vec<Diagnostic> {
    let mut validator = Validator { backend, ..Validator::default() };
    validator.actions(&patch.actions, "");

    let calls = std::mem::take(&mut validator.calls);
    for (action, field, name, count) in &calls {
        if !validator.names.contains_key(name) {
            validator.report(DiagnosticKind::ActionNotFound, action, field, format!("No action named '{name}'"));
            continue
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

    let called: HashSet<&String> = calls.iter().map(|(_, _, name, _)| name).collect();
    let mut unused: Vec<(&String, &String)> = validator.names.iter()
        .filter(|(name, _)| !called.contains(name))
        .collect();
    unused.sort_by_key(|(_, path)| path.to_string());

    let mut diagnostics = validator.diagnostics.clone();
    for (name, path) in unused {
        diagnostics.push(Diagnostic::new(
            DiagnosticKind::UnusedAction, path, "name", format!("Action '{name}' is never used")
        ));
    }

    diagnostics
}



fn join(parent: &str, action: &str) -> String {
    if parent.is_empty() { action.to_string() } else { format!("{parent}/{action}") }
}


#[derive(Clone, Copy)]
enum Locals {
    Groups(usize),
    Unavailable(&'static str),
    Unknown
}

#[derive(Default)]
struct Validator<'a> {
    backend: Option<&'a dyn LanguageBackend>,
    diagnostics: Vec<Diagnostic>,
    names: HashMap<String, String>,
    receivers: HashSet<String>,
    required: HashMap<String, usize>,
    calls: Vec<(String, &'static str, String, usize)>
}

impl Validator<'_> {
    fn report(&mut self, kind: DiagnosticKind, action: &str, field: &'static str, message: String) {
        self.diagnostics.push(Diagnostic::new(kind, action, field, message));
    }

    fn actions(&mut self, actions: &Actions, parent: &str) {
        for builder in actions.builders.iter().flatten() {
            let path = self.name(parent, &builder.name);

            self.location(&path, "location", &builder.location);
            if let Some(reference_location) = &builder.reference_location {
                self.location(&path, "reference-location", reference_location);
            }

            let locals = self.regex(&path, "match", &builder.r#match);
            self.expression(&path, "build", &builder.build, locals, None);
            self.actions(&builder.actions, &path);
        }

        for (index, replacer) in actions.replacers.iter().flatten().enumerate() {
            let path = join(parent, &format!("replacer #{}", index + 1));

            self.location(&path, "location", &replacer.location);
            if !replacer.reference_location.is_empty() {
                self.location(&path, "reference-location", &replacer.reference_location);
            }

            let locals = self.regex(&path, "from", &replacer.from);
            self.expression(&path, "to", &replacer.to, locals, None);
            self.actions(&replacer.actions, &path);
        }

        for maker in actions.makers.iter().flatten() {
            let path = self.name(parent, &maker.name);
            self.receivers.insert(maker.name.clone());

            let locals = match &maker.prepare {
                Some(prepare) => self.regex(&path, "prepare", prepare),
                None => Locals::Unavailable("the maker has no 'prepare' regex")
            };
            self.expression(&path, "make", &maker.make, locals, Some(&maker.name));
        }

        for list in actions.lists.iter().flatten() {
            let path = self.name(parent, &list.name);
            self.receivers.insert(list.name.clone());
            if let Some(index) = &list.index {
                self.expression(&path, "index", index, Locals::Unavailable("'index' has no regex"), Some(&list.name));
            }
        }

        for switch in actions.switchers.iter().flatten() {
            let path = self.name(parent, &switch.name);
            self.receivers.insert(switch.name.clone());
            self.expression(&path, "index", &switch.index, Locals::Unavailable("'index' has no regex"), Some(&switch.name));
        }
    }

    fn name(&mut self, parent: &str, name: &str) -> String {
        let path = join(parent, name);

        if let Some(first) = self.names.get(name) {
            let message = format!("Action '{name}' is already defined at '{first}', so this one is never used");
            self.report(DiagnosticKind::DuplicateAction, &path, "name", message);
        } else {
            self.names.insert(name.to_string(), path.clone());
        }

        path
    }

    fn location(&mut self, action: &str, field: &'static str, location: &str) {
        let Some(backend) = self.backend else { return };

        if let Err(error) = backend.check_location(location) {
            self.report(DiagnosticKind::LocationNotFound, action, field, error.to_string());
        }
    }

    fn regex(&mut self, action: &str, field: &'static str, regex: &str) -> Locals {
        match Regex::new(regex) {
            Ok(regex) => Locals::Groups(regex.captures_len() - 1),
            Err(error) => {
                self.report(DiagnosticKind::IncorrectRegex, action, field, error.to_string());
                Locals::Unknown
            }
        }
    }

    fn expression(&mut self, action: &str, field: &'static str, text: &str, locals: Locals, arguments: Option<&str>) {
        match parse_expression(text) {
            Ok(expression) => self.walk(action, field, &expression, locals, arguments),
            Err(error) => self.report(DiagnosticKind::IncorrectExpression, action, field, error.to_string())
        }
    }

    fn walk(
        &mut self, action: &str, field: &'static str, expression: &ExpressionAST, locals: Locals, arguments: Option<&str>
    ) {
        match expression {
            ExpressionAST::Action(parts) => for part in parts {
                self.walk(action, field, part, locals, arguments);
            },
            ExpressionAST::ActionCall((name, call_arguments)) => {
                self.calls.push((action.to_string(), field, name.to_string(), call_arguments.len()));

                for argument in call_arguments {
                    self.walk(action, field, argument, locals, arguments);
                }
            }
            ExpressionAST::LocalCall(variable) => self.local(action, field, variable, locals),
            ExpressionAST::ArgumentCall(variable) => match (arguments, variable.parse::<usize>()) {
                (None, _) => {
                    let message = format!("'@[{variable}]' is used where no arguments exist");
                    self.report(DiagnosticKind::IncorrectArgs, action, field, message);
                }
                (Some(_), Err(_)) => {
                    let message = format!("'@[{variable}]' must be an argument number");
                    self.report(DiagnosticKind::IncorrectArgs, action, field, message);
                }
                (Some(receiver), Ok(index)) => {
                    let required = self.required.entry(receiver.to_string()).or_default();
                    *required = (*required).max(index + 1);
                }
            },
            ExpressionAST::Text(_) => {}
        }
    }

    fn local(&mut self, action: &str, field: &'static str, variable: &str, locals: Locals) {
        let message = match (locals, variable.parse::<usize>()) {
            (Locals::Unknown, _) => return,
            (Locals::Unavailable(reason), _) => format!("'$[{variable}]' has no capture group to refer to: {reason}"),
            (Locals::Groups(_), Err(_)) => format!("'$[{variable}]' must be a capture group number"),
            (Locals::Groups(groups), Ok(index)) if index >= groups =>
                format!("'$[{index}]' needs {} capture group(s), but the regex has {groups}", index + 1),
            (Locals::Groups(_), Ok(_)) => return
        };

        self.report(DiagnosticKind::IncorrectArgs, action, field, message);
    }
}
