// Standard Uses
use std::collections::BTreeSet;

// Crate Uses
use crate::{to_pascal_case, to_snake_case, TreeSitterBackend};

// External Uses
use codeshaper_core::ast::backend::{Suggestion, Suggestions, SyntaxNode};
use miette::Result;
use tree_sitter::{Node, Tree};


const PATHS: usize = 4;
const ANCESTORS: usize = 8;
const NAMES: usize = 4;
const DEPTH: usize = 5;
const LENGTH: usize = 80;


struct Named<'tree> {
    node: Node<'tree>,
    field: Option<&'tree str>,
    parent: Option<usize>,
    in_error: bool
}


impl TreeSitterBackend {
    pub(crate) fn syntax_nodes(&self, tree: &Tree) -> Vec<SyntaxNode> {
        named(tree).into_iter()
            .map(|Named { node, field, parent, in_error }| SyntaxNode {
                kind: segment_name(node.kind()),
                field: field.map(segment_name),
                range: node.byte_range(),
                parent,
                broken: in_error || node.has_error() || node.is_missing()
            })
            .collect()
    }

    pub(crate) fn suggest(&self, tree: &Tree, source: &str, node: usize) -> Result<Suggestions> {
        let nodes = named(tree);
        let Some(Named { node: wanted, parent: Some(_), .. }) = nodes.get(node) else { return Ok(Suggestions::default()) };
        if wanted.is_error() || wanted.is_missing() { return Ok(Suggestions::default()) }

        let mut paths: Vec<Suggestion> = vec![];
        for location in candidates(&nodes, node) {
            if paths.len() == PATHS { break }
            if paths.iter().any(|path| path.location == location) { continue }

            let ranges = self.locate_in(tree, source, &location).unwrap_or_default();
            if ranges.contains(&wanted.byte_range()) {
                paths.push(Suggestion { location, ranges });
            }
        }

        let query = match paths.iter().any(|path| path.ranges.len() == 1) {
            true => None,
            false => self.only(tree, source, &nodes, node)
        };

        Ok(Suggestions { paths, query })
    }

    fn only(&self, tree: &Tree, source: &str, nodes: &[Named], target: usize) -> Option<Suggestion> {
        let children = children(nodes);

        for is_own in [true, false] {
            let ancestors = std::iter::successors(Some(target), |node| nodes[*node].parent)
                .filter(|node| nodes[*node].parent.is_some())
                .take(ANCESTORS);

            for ancestor in ancestors {
                for name in names(nodes, source, ancestor, target, is_own) {
                    if !is_alone(nodes, &children, source, ancestor, target, name) { continue }

                    let query = query(nodes, source, ancestor, target, name);
                    let ranges = self.locate_in(tree, source, &query).unwrap_or_default();

                    if ranges == [nodes[target].node.byte_range()] {
                        return Some(Suggestion { location: query, ranges })
                    }
                }
            }
        }

        None
    }
}


fn named(tree: &Tree) -> Vec<Named<'_>> {
    let mut found: Vec<Named> = vec![];
    let mut above: Vec<(Option<usize>, bool, bool)> = vec![];
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();
        let (parent, is_direct, in_error) = above.last().copied().unwrap_or((None, true, false));
        let in_error = in_error || node.is_error();
        let here = match node.is_named() {
            true => {
                found.push(Named { node, field: cursor.field_name().filter(|_| is_direct), parent, in_error });
                Some(found.len() - 1)
            }
            false => parent
        };

        if cursor.goto_first_child() {
            above.push((here, node.is_named(), in_error));
            continue
        }

        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() { return found }
            above.pop();
        }
    }
}

fn candidates(nodes: &[Named], node: usize) -> Vec<String> {
    let kind = segment_name(nodes[node].node.kind());
    let inner = nodes[node].parent.filter(|parent| nodes[*parent].parent.is_some());
    let parent = inner.map(|parent| segment_name(nodes[parent].node.kind()));
    let mut found = vec![];

    if let (Some(parent), Some(field)) = (&parent, nodes[node].field) {
        found.push(format!("{parent}.{}", segment_name(field)));
        found.push(chain(nodes, node));
    }
    found.push(kind.clone());
    if let (Some(parent), Some(inner), None) = (parent, inner, nodes[node].field) {
        found.push(format!("{parent}.{kind}"));
        found.push(format!("{}.{kind}", chain(nodes, inner)));
    }

    found
}

fn chain(nodes: &[Named], node: usize) -> String {
    let mut segments = vec![];
    let mut current = node;

    while let (Some(field), Some(parent)) = (nodes[current].field, nodes[current].parent) {
        if nodes[parent].parent.is_none() { break }

        segments.push(segment_name(field));
        current = parent;
    }
    segments.push(segment_name(nodes[current].node.kind()));
    segments.reverse();

    segments.join(".")
}

fn names(nodes: &[Named], source: &str, ancestor: usize, target: usize, is_own: bool) -> Vec<usize> {
    let end = nodes[ancestor].node.end_byte();
    let inside = |node: usize, outer: usize| node == outer || path(nodes, outer, node).last().is_some_and(|top| nodes[top].parent == Some(outer));
    let depth = |node: usize| path(nodes, ancestor, node).count();

    let mut found: Vec<usize> = (ancestor..nodes.len())
        .take_while(|node| *node == ancestor || nodes[*node].node.start_byte() < end)
        .filter(|node| nodes[*node].node.named_child_count() == 0 && !nodes[*node].in_error && !nodes[*node].node.is_missing())
        .filter(|node| inside(*node, ancestor) && depth(*node) <= DEPTH && (ancestor == target || !inside(*node, target)))
        .filter(|node| {
            let text = &source[nodes[*node].node.byte_range()];
            !text.is_empty() && text.len() <= LENGTH && !text.contains(['\n', '\r'])
        })
        .collect();

    found.retain(|node| is_own == (is_identifier(nodes[*node].node) && path(nodes, ancestor, *node).all(|step| nodes[step].field.is_some())));
    found.sort_by_key(|node| (depth(*node), *node));
    found.truncate(NAMES);
    found
}

fn children(nodes: &[Named]) -> Vec<Vec<usize>> {
    let mut found = vec![vec![]; nodes.len()];

    for (index, node) in nodes.iter().enumerate() {
        if let Some(parent) = node.parent { found[parent].push(index); }
    }

    found
}

fn is_alone(nodes: &[Named], children: &[Vec<usize>], source: &str, ancestor: usize, target: usize, name: usize) -> bool {
    let above: Vec<usize> = path(nodes, ancestor, target).collect();
    let mut below: Vec<usize> = path(nodes, ancestor, name).collect();
    below.reverse();

    !(0..nodes.len()).filter(|other| *other != target).any(|other| {
        let mut current = Some(other);

        for step in &above {
            current = current.filter(|current| is_like(nodes, *current, *step)).and_then(|current| nodes[current].parent);
        }

        current.is_some_and(|current| {
            nodes[current].node.kind_id() == nodes[ancestor].node.kind_id() && leads(nodes, children, source, current, &below, name)
        })
    })
}

fn leads(nodes: &[Named], children: &[Vec<usize>], source: &str, from: usize, steps: &[usize], name: usize) -> bool {
    let text = |node: usize| &source[nodes[node].node.byte_range()];

    match steps {
        [] => text(from) == text(name),
        [step, rest @ ..] => children[from].iter()
            .any(|child| is_like(nodes, *child, *step) && leads(nodes, children, source, *child, rest, name))
    }
}

fn is_like(nodes: &[Named], node: usize, pattern: usize) -> bool {
    nodes[node].node.kind_id() == nodes[pattern].node.kind_id()
        && (nodes[pattern].field.is_none() || nodes[node].field == nodes[pattern].field)
}

fn path<'n>(nodes: &'n [Named], ancestor: usize, node: usize) -> impl Iterator<Item = usize> + 'n {
    std::iter::successors(Some(node), |node| nodes[*node].parent).take_while(move |node| *node != ancestor)
}

fn is_identifier(node: Node) -> bool {
    node.kind().contains("identifier") || node.kind().contains("name")
}

fn query(nodes: &[Named], source: &str, ancestor: usize, target: usize, name: usize) -> String {
    let within: BTreeSet<usize> = path(nodes, ancestor, target).chain(path(nodes, ancestor, name)).chain([ancestor]).collect();
    let capture = if name == target { "target" } else { "name" };
    let text = source[nodes[name].node.byte_range()].replace('\\', "\\\\").replace('"', "\\\"").replace('\t', "\\t");
    let pattern = pattern(nodes, &within, ancestor, target, name);

    format!("({pattern} (#eq? @{capture} \"{text}\"))")
}

fn pattern(nodes: &[Named], within: &BTreeSet<usize>, node: usize, target: usize, name: usize) -> String {
    let children: String = within.iter()
        .filter(|child| nodes[**child].parent == Some(node))
        .map(|child| {
            let field = nodes[*child].field.map(|field| format!("{field}: ")).unwrap_or_default();
            format!(" {field}{}", pattern(nodes, within, *child, target, name))
        })
        .collect();
    let capture = match node {
        _ if node == target => " @target",
        _ if node == name => " @name",
        _ => ""
    };

    format!("({}{children}){capture}", nodes[node].node.kind())
}

fn segment_name(name: &str) -> String {
    let pascal = to_pascal_case(name);

    if to_snake_case(&pascal) == name { pascal } else { name.to_string() }
}
