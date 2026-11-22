// Standard Uses

// Crate Uses

// External Uses
use codeshaper_lsp::tokens::tokens;


fn absolute(text: &str) -> Vec<(u32, u32, u32, u32)> {
    let mut line = 0;
    let mut start = 0;

    tokens(text).into_iter()
        .map(|token| {
            line += token.delta_line;
            start = if token.delta_line == 0 { start + token.delta_start } else { token.delta_start };
            (line, start, token.length, token.token_type)
        })
        .collect()
}


#[test]
fn expressions_inside_strings_are_tokens() {
    let text = "builder \"b\" {\n    build \"ä $[0] #[pair]($[0], @[1])\"\n    // #[comment]\n}\n";

    assert_eq!(absolute(text), vec![
        (1, 13, 4, 1),
        (1, 18, 7, 0),
        (1, 26, 4, 1),
        (1, 32, 4, 2),
    ]);
}

#[test]
fn unterminated_and_empty_markers_are_skipped() {
    let text = "make \"#[] $[ @[x\"";

    assert_eq!(absolute(text), vec![(0, 13, 3, 2)]);
}
