// Standard Uses

// Crate Uses
use crate::position::LineIndex;
use crate::section::views;
use crate::syntax::{Node, Outline, ACTION_NODES};
use crate::values::KINDS;

// External Uses
use codeshaper::core::shaping::patch::file::BLOCK;
use lsp_types::{DocumentSymbol, FoldingRange, FoldingRangeKind, SymbolKind};


const FIELDS: [&str; 3] = ["enabled", "alias", "file"];


#[allow(deprecated)]
pub fn symbols(text: &str) -> Vec<DocumentSymbol> {
    let index = LineIndex::new(text);

    views(text).into_iter()
        .flat_map(|view| {
            let found = patch(&view.text, &index);
            let Some(section) = view.section else { return found };

            vec![DocumentSymbol {
                name: section.name.filter(|name| !name.is_empty()).unwrap_or(BLOCK.to_string()),
                detail: Some(BLOCK.to_string()),
                kind: SymbolKind::NAMESPACE,
                tags: None,
                deprecated: None,
                range: index.range(section.whole),
                selection_range: index.range(section.named),
                children: (!found.is_empty()).then_some(found)
            }]
        })
        .collect()
}

#[allow(deprecated)]
fn patch(text: &str, index: &LineIndex) -> Vec<DocumentSymbol> {
    let outline = Outline::parse(text);

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

    let values = outline.nodes.iter()
        .filter(|node| node.parent.is_none() && !node.commented && KINDS.contains(&node.name.as_str()))
        .filter_map(|node| {
            let name = node.arguments.first()?;

            Some(DocumentSymbol {
                name: name.value.clone(),
                detail: Some(node.name.clone()),
                kind: if node.name == KINDS[0] { SymbolKind::CONSTANT } else { SymbolKind::VARIABLE },
                tags: None,
                deprecated: None,
                range: index.range(node.name_range.start..end(text, node)),
                selection_range: index.range(name.range.clone()),
                children: None
            })
        })
        .filter(|symbol| !symbol.name.is_empty());

    fields.chain(values).chain(actions(&outline, text, index, None)).collect()
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
