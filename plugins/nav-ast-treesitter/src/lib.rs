// Standard Uses
use std::ops::Range;

// Crate Uses

// External Uses
use codeshaper_core::ast::backend::{LanguageBackend, LocationError};
use codeshaper_core::registry::Registry;
use miette::{miette, bail, Report, Result};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, QueryError, QueryErrorKind, StreamingIterator, Tree};


#[allow(unused_variables)]
pub fn register(registry: &mut Registry) {
    #[cfg(feature = "cpp")]
    registry.add_backend(Box::new(TreeSitterBackend::cpp()));

    #[cfg(feature = "gdscript")]
    registry.add_backend(Box::new(TreeSitterBackend::gdscript()));

    #[cfg(feature = "godot-resource")]
    registry.add_backend(Box::new(TreeSitterBackend::godot_resource()));
}


pub struct TreeSitterBackend {
    name: String,
    language: Language,
}

impl TreeSitterBackend {
    pub fn new(name: &str, language: Language) -> Self {
        Self { name: name.to_string(), language }
    }

    #[cfg(feature = "cpp")]
    pub fn cpp() -> Self {
        Self::new("cpp", tree_sitter_cpp::LANGUAGE.into())
    }

    #[cfg(feature = "gdscript")]
    pub fn gdscript() -> Self {
        Self::new("gdscript", tree_sitter_gdscript::LANGUAGE.into())
    }

    #[cfg(feature = "godot-resource")]
    pub fn godot_resource() -> Self {
        Self::new("godot_resource", tree_sitter_godot_resource::LANGUAGE.into())
    }

    fn parse(&self, source: &str) -> Result<Tree> {
        let mut parser = Parser::new();
        parser.set_language(&self.language).map_err(|err| miette!("{err}"))?;

        parser.parse(source, None)
            .ok_or_else(|| miette!("Could not parse source as {}", self.name))
    }

    fn query(&self, query: &str) -> Result<(Query, u32)> {
        let query = Query::new(&self.language, query).map_err(|error| self.query_error(query, error))?;
        let Some(target) = query.capture_index_for_name("target") else {
            bail!("Query location has no @target capture")
        };

        Ok((query, target))
    }

    fn query_error(&self, query: &str, error: QueryError) -> LocationError {
        let name = error.message.trim_matches('"');
        let message = match error.kind {
            QueryErrorKind::NodeType => format!("'{name}' is not a {} node kind", self.name),
            QueryErrorKind::Field => format!("'{name}' is not a {} field", self.name),
            QueryErrorKind::Capture => format!("'@{name}' is not captured in the query"),
            QueryErrorKind::Predicate => format!("Invalid predicate: {}", error.message),
            QueryErrorKind::Structure => format!("This pattern never matches in {}", self.name),
            QueryErrorKind::Syntax => "Invalid query syntax".to_string(),
            QueryErrorKind::Language => error.message
        };

        let range = match error.kind {
            QueryErrorKind::Predicate | QueryErrorKind::Language => None,
            _ => query.get(error.offset..).map(|rest| {
                let length = rest.find(|character: char| !is_word(character)).unwrap_or(rest.len());
                error.offset..error.offset + length
            })
        };

        LocationError { message, range }
    }

    fn segment(&self, index: usize, segment: &str) -> Result<Segment> {
        let segment = to_snake_case(segment.trim());

        if index > 0 && self.language.field_id_for_name(&segment).is_some() {
            Ok(Segment::Field(segment))
        } else if self.language.id_for_node_kind(&segment, true) != 0 {
            Ok(Segment::Kind(segment))
        } else {
            bail!("'{}' is not a {} node kind or field", segment, self.name)
        }
    }

    fn locate_query(&self, tree: &Tree, source: &str, query: &str) -> Result<Vec<Range<usize>>> {
        let (query, target) = self.query(query)?;

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
            let mut next = vec![];

            match self.segment(index, segment)? {
                Segment::Field(field) => for node in &nodes {
                    let mut cursor = node.walk();
                    next.extend(node.children_by_field_name(&field, &mut cursor));
                },
                Segment::Kind(kind) => for node in &nodes {
                    collect_descendants(*node, &kind, &mut next);
                }
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

    fn check_location(&self, location: &str) -> Result<()> {
        let leading = location.len() - location.trim_start().len();
        let location = location.trim();

        if location.starts_with('(') {
            return self.query(location).map(|_| ()).map_err(|error| shift(error, leading))
        }

        let mut start = leading;
        for (index, segment) in location.split('.').enumerate() {
            let range = start..start + segment.len();
            self.segment(index, segment).map_err(|error| LocationError { message: error.to_string(), range: Some(range.clone()) })?;
            start = range.end + 1;
        }

        Ok(())
    }

    fn describe_location(&self, location: &str) -> Result<String> {
        let location = location.trim();

        if location.starts_with('(') {
            self.query(location)?;
            return Ok(format!("{} query, located at its `@target` capture", self.name))
        }

        let segments = location.split('.').enumerate()
            .map(|(index, segment)| Ok(match self.segment(index, segment)? {
                Segment::Field(field) => format!("field `{field}`"),
                Segment::Kind(kind) => format!("node kind `{kind}`")
            }))
            .collect::<Result<Vec<_>>>()?;

        Ok(format!("{}: {}", self.name, segments.join(" › ")))
    }

    fn node_kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = (0..self.language.node_kind_count() as u16)
            .filter(|id| self.language.node_kind_is_named(*id) && self.language.node_kind_is_visible(*id))
            .filter_map(|id| self.language.node_kind_for_id(id))
            .filter(|kind| !kind.starts_with('_'))
            .map(str::to_string)
            .collect();

        kinds.sort();
        kinds.dedup();
        kinds
    }

    fn fields(&self) -> Vec<String> {
        (1..=self.language.field_count() as u16)
            .filter_map(|id| self.language.field_name_for_id(id))
            .map(str::to_string)
            .collect()
    }

    fn location_names(&self, segment: usize) -> Vec<String> {
        let mut names: Vec<String> = self.node_kinds().iter().map(|kind| to_pascal_case(kind)).collect();

        if segment > 0 {
            names.extend(self.fields().iter().map(|field| to_pascal_case(field)));
        }

        names.sort();
        names.dedup();
        names
    }
}


enum Segment {
    Field(String),
    Kind(String)
}


fn shift(error: Report, by: usize) -> Report {
    match error.downcast::<LocationError>() {
        Ok(LocationError { message, range }) => {
            LocationError { message, range: range.map(|range| range.start + by..range.end + by) }.into()
        }
        Err(error) => error
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.' | '?' | '!')
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

fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut characters = word.chars();
            characters.next().map(|first| first.to_uppercase().chain(characters).collect::<String>()).unwrap_or_default()
        })
        .collect()
}
