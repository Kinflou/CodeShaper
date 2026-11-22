// Standard Uses

// Crate Uses

// External Uses
use codeshaper_lsp::navigation::{definition, references};
use codeshaper_lsp::syntax::Outline;


const PATCH: &str = r##"actions {
    builders {
        builder "names" {
            match r"(\w+)"
            build "$[0] #[pair]($[0])"
        }
    }
    makers {
        maker "pair" {
            make "@[0]=#[names]"
        }
    }
    replacers {
        replacer {
            to "#[pair](x) #[names]"
        }
    }
}
"##;

fn nth(needle: &str, n: usize) -> usize {
    PATCH.match_indices(needle).nth(n).unwrap().0
}


#[test]
fn definition_from_calls_and_names() {
    let outline = Outline::parse(PATCH);
    let pair = nth("pair", 1);

    assert_eq!(definition(&outline, PATCH, nth("#[pair]", 0) + 3), Some(pair..pair + 4));
    assert_eq!(definition(&outline, PATCH, pair + 2), Some(pair..pair + 4));
    assert_eq!(definition(&outline, PATCH, nth("match", 0)), None);
}

#[test]
fn references_with_and_without_the_declaration() {
    let outline = Outline::parse(PATCH);
    let at = nth("#[names]", 0) + 4;

    let calls = references(&outline, PATCH, at, false);
    assert_eq!(calls.iter().map(|range| &PATCH[range.clone()]).collect::<Vec<_>>(), vec!["names", "names"]);
    assert_eq!(references(&outline, PATCH, at, true).len(), 3);
}
