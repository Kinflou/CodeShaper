// Standard Uses
use std::iter;

// Crate Uses

// External Uses
use codeshaper_lsp::symbols::{folding, symbols};
use lsp_types::{DocumentSymbol, SymbolKind};


const PATCH: &str = r##"enabled true
alias ""

actions {
    builder "names" {
        build "x"
        actions {
            maker "inner" {
                make "y"
            }
        }
    }
    replacer {
        to "#[names]"
    }
    replacer {
        to "z"
    }
}
"##;


fn tree(symbols: &[DocumentSymbol]) -> Vec<String> {
    symbols.iter()
        .flat_map(|symbol| {
            let children = tree(symbol.children.as_deref().unwrap_or_default());
            iter::once(format!("{} ({:?})", symbol.name, symbol.kind))
                .chain(children.into_iter().map(|child| format!("  {child}")))
        })
        .collect()
}


#[test]
fn symbols_follow_the_action_tree() {
    let found = symbols(PATCH);

    assert_eq!(tree(&found), vec![
        format!("enabled ({:?})", SymbolKind::PROPERTY),
        format!("alias ({:?})", SymbolKind::PROPERTY),
        format!("names ({:?})", SymbolKind::FUNCTION),
        format!("  inner ({:?})", SymbolKind::FUNCTION),
        format!("replacer #1 ({:?})", SymbolKind::METHOD),
        format!("replacer #2 ({:?})", SymbolKind::METHOD),
    ]);
    assert_eq!((found[2].range.start.line, found[2].range.end.line), (4, 11));
    assert_eq!(found[2].selection_range.start.character, 12);
}

#[test]
fn symbols_keep_the_written_order() {
    let text = "actions {\n    replacer {\n        actions {\n            list \"l\" {\n            }\n            maker \"m\" {\n            }\n        }\n    }\n    \
        switch \"s\" {\n    }\n    replacer {\n    }\n    builder \"b\" {\n    }\n}\n";
    let names: Vec<String> = tree(&symbols(text)).iter().map(|line| line.split(" (").next().unwrap().to_string()).collect();

    assert_eq!(names, vec!["replacer #1", "  l", "  m", "s", "replacer #2", "b"]);
}

#[test]
fn folding_covers_multiline_blocks() {
    let lines: Vec<(u32, u32)> = folding(PATCH).iter().map(|range| (range.start_line, range.end_line)).collect();

    assert_eq!(lines, vec![(3, 18), (4, 11), (6, 10), (7, 9), (12, 14), (15, 17)]);
}

#[test]
fn replacers_are_shown_by_name() {
    let found = symbols(&PATCH.replacen("    replacer {", "    replacer \"header\" {", 1));

    assert_eq!(tree(&found)[4..], [format!("header ({:?})", SymbolKind::METHOD), format!("replacer #2 ({:?})", SymbolKind::METHOD)]);
    assert_eq!((found[3].selection_range.start.character, found[3].selection_range.end.character), (13, 21));
}
