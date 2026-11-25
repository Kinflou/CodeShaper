// Standard Uses

// Crate Uses

// External Uses
use codeshaper_lsp::navigation::{definition, references, rename};
use codeshaper_lsp::syntax::Outline;


const PATCH: &str = r##"actions {
    builder "names" {
        match r"(\w+)"
        build "$[0] #[pair($[0])]"
    }
    maker "pair" {
        make "@[0]=#[names]"
    }
    replacer {
        to "#[pair](x) #[names]"
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

#[test]
fn rename_covers_the_definition_and_every_call() {
    let outline = Outline::parse(PATCH);
    let mut ranges = rename(&outline, PATCH, nth("#[pair]", 0) + 3, "couple").unwrap();
    ranges.sort_by_key(|range| std::cmp::Reverse(range.start));

    let mut renamed = PATCH.to_string();
    for range in ranges {
        renamed.replace_range(range, "couple");
    }

    assert_eq!(renamed.matches("couple").count(), 3);
    assert!(!renamed.contains("pair"));
}

#[test]
fn rename_refuses_invalid_names_and_plain_text() {
    let outline = Outline::parse(PATCH);

    assert_eq!(rename(&outline, PATCH, nth("#[pair]", 0) + 3, "two words"), None);
    assert_eq!(rename(&outline, PATCH, nth("#[pair]", 0) + 3, ""), None);
    assert_eq!(rename(&outline, PATCH, nth("x)", 0), "other"), None);
}

#[test]
fn calls_inside_arguments_are_followed() {
    let text = "maker \"a\" {\n}\nreplacer {\n    to \"#[b('#[a]', #[a(1)])]\"\n}\n";
    let outline = Outline::parse(text);
    let found = references(&outline, text, text.find("#[a(1)").unwrap() + 2, true);

    assert_eq!(found.iter().map(|range| range.start).collect::<Vec<_>>(), [7, text.find("#[a]").unwrap() + 2, text.find("#[a(").unwrap() + 2]);
    assert_eq!(definition(&outline, text, text.find("#[a]").unwrap() + 3), Some(7..8));
}

#[test]
fn replacers_are_renamed_by_name() {
    let text = "actions {\n    replacer \"first\" {\n        to \"#[second] #[m]\"\n    }\n    replacer \"second\" {\n        to \"first\"\n    }\n    maker \"m\" {\n    }\n}\n";
    let outline = Outline::parse(text);
    let at = |needle: &str| text.find(needle).unwrap();
    let spans = |ranges: Vec<std::ops::Range<usize>>| ranges.iter().map(|range| range.start).collect::<Vec<_>>();

    assert_eq!(rename(&outline, text, at("first") + 1, "x").map(spans), Some(vec![at("first")]));
    assert_eq!(rename(&outline, text, at("second\" {"), "x").map(spans), Some(vec![at("second\" {"), at("second]")]));
    assert_eq!(definition(&outline, text, at("second]") + 2), Some(at("second\" {")..at("second\" {") + 6));
    assert_eq!(spans(references(&outline, text, at("second\" {"), false)), vec![at("second]")]);
    assert_eq!(rename(&outline, text, at("\"first\"\n") + 2, "x"), None);
}
