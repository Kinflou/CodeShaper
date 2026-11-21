use std::ops::Range;

use codeshaper_core::ast::backend::LanguageBackend;

use miette::{miette, bail, Result};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator, Tree};


pub struct TreeSitterBackend {
    name: String,
    language: Language,
}

impl TreeSitterBackend {
    pub fn new(name: &str, language: Language) -> Self {
        Self { name: name.to_string(), language }
    }

    pub fn cpp() -> Self {
        Self::new("cpp", tree_sitter_cpp::LANGUAGE.into())
    }

    fn parse(&self, source: &str) -> Result<Tree> {
        let mut parser = Parser::new();
        parser.set_language(&self.language).map_err(|err| miette!("{err}"))?;

        parser.parse(source, None)
            .ok_or_else(|| miette!("Could not parse source as {}", self.name))
    }

    fn locate_query(&self, tree: &Tree, source: &str, query: &str) -> Result<Vec<Range<usize>>> {
        let query = Query::new(&self.language, query).map_err(|err| miette!("{err}"))?;
        let Some(target) = query.capture_index_for_name("target") else {
            bail!("Query location has no @target capture")
        };

        let mut ranges = vec![];
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        while let Some(found) = matches.next() {
            ranges.extend(found.nodes_for_capture_index(target).map(|node| node.byte_range()));
        }

        Ok(ranges)
    }

    fn locate_path(&self, tree: &Tree, path: &str) -> Result<Vec<Range<usize>>> {
        let mut nodes = vec![tree.root_node()];

        for (index, segment) in path.split('.').enumerate() {
            let segment = to_snake_case(segment.trim());
            let mut next = vec![];

            if index > 0 && self.language.field_id_for_name(&segment).is_some() {
                for node in &nodes {
                    let mut cursor = node.walk();
                    next.extend(node.children_by_field_name(&segment, &mut cursor));
                }
            } else if self.language.id_for_node_kind(&segment, true) != 0 {
                for node in &nodes {
                    collect_descendants(*node, &segment, &mut next);
                }
            } else {
                bail!("'{}' is not a {} node kind or field", segment, self.name)
            }

            nodes = next;
        }

        Ok(nodes.iter().map(|node| node.byte_range()).collect())
    }
}

impl LanguageBackend for TreeSitterBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn locate(&self, source: &str, location: &str) -> Result<Vec<Range<usize>>> {
        let tree = self.parse(source)?;
        let location = location.trim();

        let mut ranges = if location.starts_with('(') {
            self.locate_query(&tree, source, location)?
        } else {
            self.locate_path(&tree, location)?
        };

        ranges.sort_by_key(|range| (range.start, range.end));
        ranges.dedup();

        Ok(ranges)
    }
}


fn collect_descendants<'tree>(node: Node<'tree>, kind: &str, found: &mut Vec<Node<'tree>>) {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return
    }

    let mut depth = 1;
    loop {
        if cursor.node().kind() == kind {
            found.push(cursor.node());
        }

        if cursor.goto_first_child() {
            depth += 1;
            continue
        }

        while !cursor.goto_next_sibling() {
            cursor.goto_parent();
            depth -= 1;
            if depth == 0 {
                return
            }
        }
    }
}

fn to_snake_case(name: &str) -> String {
    let mut snake = String::with_capacity(name.len() + 4);

    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 {
                snake.push('_');
            }
            snake.extend(character.to_lowercase());
        } else {
            snake.push(character);
        }
    }

    snake
}
