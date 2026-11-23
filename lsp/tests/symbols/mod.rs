// Standard Uses
use std::iter;

// Crate Uses

// External Uses
use codeshaper_lsp::symbols::{folding, symbols};
use lsp_types::{DocumentSymbol, SymbolKind};


const PATCH: &str = r##"enabled true
alias "cpp"

actions {
    builders {
        builder "names" {
            build "x"
            actions {
                makers {
                    maker "inner" {
                        make "y"
                    }
                }
            }
        }
    }
    replacers {
        replacer {
            to "#[names]"
        }
        replacer {
            to "z"
        }
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
    assert_eq!((found[2].range.start.line, found[2].range.end.line), (5, 14));
    assert_eq!(found[2].selection_range.start.character, 16);
}

#[test]
fn folding_covers_multiline_blocks() {
    let lines: Vec<(u32, u32)> = folding(PATCH).iter().map(|range| (range.start_line, range.end_line)).collect();

    assert_eq!(lines, vec![(3, 24), (4, 15), (5, 14), (7, 13), (8, 12), (9, 11), (16, 23), (17, 19), (20, 22)]);
}
