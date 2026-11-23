// Standard Uses
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Range;

// Crate Uses
use crate::shaping::patch::validation::{Diagnostic, Severity};

// External Uses
use knus::ast::SpannedNode;
use knus::span::Span;
use miette::{LabeledSpan, NamedSource, Report, Result, SourceCode, SourceSpan};


#[derive(Debug, Clone)]
pub struct PatchSource {
    pub name: String,
    pub text: String,
    pub spans: NodeSpans
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct NodeSpans {
    pub fields: HashMap<String, SourceSpan>,
    pub actions: ActionSpans
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ActionSpans {
    pub builders: Vec<NodeSpans>,
    pub replacers: Vec<NodeSpans>,
    pub makers: Vec<NodeSpans>,
    pub switchers: Vec<NodeSpans>,
    pub lists: Vec<NodeSpans>
}

impl PatchSource {
    pub fn parse(name: &str, text: &str) -> Result<Self> {
        let document = knus::parse_ast::<Span>(name, text).map_err(Report::new)?;

        Ok(Self { name: name.to_string(), text: text.to_string(), spans: node_spans(&document.nodes) })
    }

    pub fn narrow(&self, literal: SourceSpan, range: Range<usize>) -> Option<SourceSpan> {
        let text = self.text.get(literal.offset()..literal.offset() + literal.len())?;
        let offsets = literal_offsets(text);

        let start = *offsets.get(range.start)?;
        let end = *offsets.get(range.end)?;
        let (start, end) = match (start == end, text[end..].chars().next().filter(|_| range.end + 1 < offsets.len())) {
            (false, _) => (start, end),
            (true, Some(next)) => (start, end + next.len_utf8()),
            (true, None) => (*offsets.get(range.start.checked_sub(1)?)?, end)
        };

        Some(SourceSpan::from(literal.offset() + start..literal.offset() + end))
    }

    pub fn report(&self, diagnostic: &Diagnostic, title: String) -> Report {
        Report::new(SourceReport {
            title,
            severity: diagnostic.severity,
            label: diagnostic.span.map(|span| LabeledSpan::new_with_span(Some(diagnostic.field.to_string()), span)),
            source: NamedSource::new(&self.name, self.text.clone())
        })
    }
}


fn literal_offsets(text: &str) -> Vec<usize> {
    if let Some(raw) = text.strip_prefix('r') {
        let hashes = raw.bytes().take_while(|byte| *byte == b'#').count();
        let start = hashes + 2;

        return (start..=text.len().saturating_sub(hashes + 1)).collect()
    }

    let end = text.len().saturating_sub(1);
    let mut offsets = vec![];
    let mut at = 1;

    while at < end {
        let character = text[at..].chars().next().unwrap_or_default();
        let (source, decoded) = match (character, text[at + character.len_utf8()..].chars().next()) {
            ('\\', Some('u')) => {
                let length = text[at..].find('}').map(|found| found + 1).unwrap_or(2);
                let code = text[at..at + length].trim_start_matches("\\u{").trim_end_matches('}');
                let decoded = u32::from_str_radix(code, 16).ok().and_then(char::from_u32).map(char::len_utf8);

                (length, decoded.unwrap_or(1))
            }
            ('\\', Some(escaped)) => (1 + escaped.len_utf8(), escaped.len_utf8()),
            _ => (character.len_utf8(), character.len_utf8())
        };

        offsets.extend((0..decoded).map(|byte| if source == decoded { at + byte } else { at }));
        at += source;
    }

    offsets.push(end);
    offsets
}

fn source_span(span: &Span) -> SourceSpan {
    SourceSpan::from(span.0..span.1)
}

fn node_spans(nodes: &[SpannedNode<Span>]) -> NodeSpans {
    let mut spans = NodeSpans::default();

    for node in nodes {
        let name: &str = &node.node_name;

        if name == "actions" {
            spans.actions = action_spans(children(node));
            continue
        }

        let span = node.arguments.first().map(|argument| argument.literal.span()).unwrap_or(node.span());
        spans.fields.insert(name.to_string(), source_span(span));
    }

    spans
}

fn action_spans(containers: &[SpannedNode<Span>]) -> ActionSpans {
    let mut spans = ActionSpans::default();

    for container in containers {
        let (list, item) = match &**container.node_name {
            "builders" => (&mut spans.builders, "builder"),
            "replacers" => (&mut spans.replacers, "replacer"),
            "makers" => (&mut spans.makers, "maker"),
            "switchers" => (&mut spans.switchers, "switch"),
            "lists" => (&mut spans.lists, "list"),
            _ => continue
        };

        for node in children(container).iter().filter(|node| &**node.node_name == item) {
            let mut action = node_spans(children(node));

            if let Some(argument) = node.arguments.first() {
                action.fields.insert("name".to_string(), source_span(argument.literal.span()));
            }

            list.push(action);
        }
    }

    spans
}

fn children(node: &SpannedNode<Span>) -> &[SpannedNode<Span>] {
    node.children.as_ref().map(|children| children.as_slice()).unwrap_or_default()
}


#[derive(Debug)]
struct SourceReport {
    title: String,
    severity: Severity,
    label: Option<LabeledSpan>,
    source: NamedSource<String>
}

impl Display for SourceReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl std::error::Error for SourceReport {}

impl miette::Diagnostic for SourceReport {
    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.severity {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning
        })
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.label.clone().map(|label| Box::new(std::iter::once(label)) as Box<dyn Iterator<Item = LabeledSpan>>)
    }
}
