// Standard Uses

// Crate Uses
use crate::position::LineIndex;
use crate::syntax::{Node, Outline, ACTION_NODES};

// External Uses
use lsp_types::{DocumentSymbol, FoldingRange, FoldingRangeKind, SymbolKind};


const FIELDS: [&str; 3] = ["enabled", "alias", "file"];


#[allow(deprecated)]
pub fn symbols(text: &str) -> Vec<DocumentSymbol> {
    let outline = Outline::parse(text);
    let index = LineIndex::new(text);

    let fields = outline.nodes.iter()
        .filter(|node| node.parent.is_none() && FIELDS.contains(&node.name.as_str()))
        .map(|node| DocumentSymbol {
            name: node.name.clone(),
            detail: node.arguments.first().map(|literal| literal.value.clone()),
            kind: SymbolKind::PROPERTY,
            tags: None,
            deprecated: None,
            range: index.range(node.name_range.start..end(text, node)),
            selection_range: index.range(node.name_range.clone()),
            children: None
        });

    fields.chain(actions(&outline, text, &index, None)).collect()
}

#[allow(deprecated)]
fn actions(outline: &Outline, text: &str, index: &LineIndex, owner: Option<usize>) -> Vec<DocumentSymbol> {
    outline.nodes.iter().enumerate()
        .filter(|(position, node)| ACTION_NODES.contains(&node.name.as_str()) && outline.owner(*position) == owner)
        .filter_map(|(position, node)| {
            let path = outline.action_path(position)?;
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let selection = node.arguments.first().map(|literal| literal.range.clone()).unwrap_or(node.name_range.clone());
            let children = actions(outline, text, index, Some(position));

            Some(DocumentSymbol {
                name,
                detail: Some(node.name.clone()),
                kind: if node.name == "replacer" { SymbolKind::METHOD } else { SymbolKind::FUNCTION },
                tags: None,
                deprecated: None,
                range: index.range(node.name_range.start..end(text, node)),
                selection_range: index.range(selection),
                children: (!children.is_empty()).then_some(children)
            })
        })
        .collect()
}

pub fn folding(text: &str) -> Vec<FoldingRange> {
    let outline = Outline::parse(text);
    let index = LineIndex::new(text);

    outline.nodes.iter()
        .filter_map(|node| {
            let start = index.position(node.name_range.start).line;
            let end = index.position(node.children.as_ref()?.end).line;

            (end > start).then_some(FoldingRange {
                start_line: start,
                end_line: end,
                kind: Some(FoldingRangeKind::Region),
                ..FoldingRange::default()
            })
        })
        .collect()
}

fn end(text: &str, node: &Node) -> usize {
    match &node.children {
        Some(children) => (children.end + 1).min(text.len()),
        None => node.arguments.last().map(|literal| literal.range.end).unwrap_or(node.name_range.end)
    }
}
