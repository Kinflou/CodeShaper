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
    let text = "builder \"b\" {\n    build \"ä $[0] #[pair($[0], @[1])]\"\n    // #[comment]\n}\n";

    assert_eq!(absolute(text), vec![
        (1, 13, 4, 1),
        (1, 18, 6, 0),
        (1, 25, 4, 1),
        (1, 31, 4, 2),
    ]);
}

#[test]
fn calls_inside_arguments_are_tokens() {
    let text = "to \"#[a('é #[b]$[0]', #[c(1)])]()\"";

    assert_eq!(absolute(text), vec![(0, 4, 3, 0), (0, 11, 4, 0), (0, 15, 4, 1), (0, 22, 3, 0)]);
}

#[test]
fn builtins_are_left_to_the_grammar() {
    assert_eq!(absolute("to \"#[file.path]#[a]#[case.upper(#[b])]\""), vec![(0, 16, 4, 0), (0, 33, 4, 0)]);
}

#[test]
fn unterminated_and_empty_markers_are_skipped() {
    let text = "make \"#[] $[ @[x\"";

    assert_eq!(absolute(text), vec![(0, 13, 3, 2)]);
}

#[test]
fn locations_are_types_and_properties() {
    let text = "location \"function_definition.body\"\nreference-location \"Class\"\nname \"Class.body\"\n";

    assert_eq!(absolute(text), vec![
        (0, 10, 19, 3),
        (0, 30, 4, 4),
        (1, 20, 5, 3),
    ]);
}

#[test]
fn queries_and_unfinished_segments_are_not_types() {
    assert_eq!(absolute("location r#\"(function_definition body: (_) @target)\"#\n"), vec![]);
    assert_eq!(absolute("location \"function_definition.\"\n"), vec![(0, 10, 19, 3)]);
}
