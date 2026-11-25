// Standard Uses

// Crate Uses
use codeshaper_core::expressions::syntax;
use codeshaper_core::expressions::syntax::ExpressionAST;

// External Uses


#[allow(unused)]
#[test]
fn parse_simple_expression() {
    let some_expression = "#[some_expr(\"some\", \"value\")]";

    let expression = syntax::parser::parse_expression(some_expression).unwrap();

    let expected_result = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall { name: "some_expr", arguments: vec![
            ExpressionAST::Text("some"),
            ExpressionAST::Text("value")
        ], span: 0..11 },
    ]);

    pretty_assertions::assert_eq!(expected_result, expression);
}


#[allow(unused)]
#[test]
fn parse_nested_expression() {
    let nexted_expression = "#[parent_expr(\"value_1\", #[child_expr], \"value_3\")]";

    let expression = syntax::parser::parse_expression(nexted_expression).unwrap();

    let expected_result = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall { name: "parent_expr", arguments: vec![
            ExpressionAST::Text("value_1"),
            ExpressionAST::ActionCall { name: "child_expr", arguments: vec![], span: 25..38 },
            ExpressionAST::Text("value_3")
        ], span: 0..13 },
    ]);

    pretty_assertions::assert_eq!(expression, expected_result);
}


#[allow(unused)]
#[test]
fn parse_multiple_nested_expression() {
    let nexted_expression = "#[parent_expr(\"value_1\", #[child_expr(#[child_child_expr])], \"value_3\")]";

    let expression = syntax::parser::parse_expression(nexted_expression).unwrap();

    let expected_result = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall { name: "parent_expr", arguments: vec![
            ExpressionAST::Text("value_1"),
            ExpressionAST::ActionCall { name: "child_expr", arguments: vec![
                ExpressionAST::ActionCall { name: "child_child_expr", arguments: vec![], span: 38..57 },
            ], span: 25..37 },
            ExpressionAST::Text("value_3"),
        ], span: 0..13 },
    ]);

    pretty_assertions::assert_eq!(expected_result, expression);
}


#[test]
fn unclosed_calls_are_errors() {
    for input in ["$[0]\n#[method_call_builder", "$[0", "a @[", "#[]", "#[name with space]"] {
        let error = syntax::parser::parse_expression(input).unwrap_err();

        assert!(error.to_string().contains("is not a complete call"), "{input}: {error}");
    }

    let error = syntax::parser::parse_expression("x #[method_call_builder").unwrap_err();
    assert!(error.to_string().starts_with("'#[method_call_builder' is not a complete call"), "{error}");
}

#[test]
fn brackets_without_markers_are_text() {
    for input in ["[x]", "a[0] = b[1];", "$5 and @export", "#include <std>", ""] {
        let expression = syntax::parser::parse_expression(input).unwrap();
        let expected = if input.is_empty() { vec![] } else { vec![ExpressionAST::Text(input)] };

        pretty_assertions::assert_eq!(ExpressionAST::Action(expected), expression);
    }
}

#[test]
fn text_after_a_call_is_text() {
    for (input, name, rest) in [("#[a]()", "a", "()"), ("#[a](x)", "a", "(x)"), ("#[a](\"x\")", "a", "(\"x\")")] {
        let expected = ExpressionAST::Action(vec![
            ExpressionAST::ActionCall { name, arguments: vec![], span: 0..4 }, ExpressionAST::Text(rest)
        ]);

        pretty_assertions::assert_eq!(syntax::parser::parse_expression(input).unwrap(), expected);
    }

    let expected = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall { name: "a", arguments: vec![], span: 0..4 },
        ExpressionAST::Text("("),
        ExpressionAST::LocalCall { name: "0", span: 5..9 },
        ExpressionAST::Text(")")
    ]);
    pretty_assertions::assert_eq!(syntax::parser::parse_expression("#[a]($[0])").unwrap(), expected);
}

#[test]
fn arguments_are_written_in_every_form() {
    let arguments = |input: &'static str| match syntax::parser::parse_expression(input).unwrap() {
        ExpressionAST::Action(parts) => match parts.into_iter().next().unwrap() {
            ExpressionAST::ActionCall { arguments, .. } => arguments,
            part => panic!("{input}: {part:?}")
        },
        part => panic!("{input}: {part:?}")
    };
    let text = |texts: &[&'static str]| texts.iter().map(|text| ExpressionAST::Text(text)).collect::<Vec<_>>();

    assert_eq!(arguments("#[a]"), vec![]);
    assert_eq!(arguments("#[a()]"), vec![]);
    assert_eq!(arguments("#[a( \n )]"), vec![]);
    assert_eq!(arguments("#[a('x', \"y\")]"), text(&["x", "y"]));
    assert_eq!(arguments("#[a(\r\n\t'x'\r\n\t,'y'\r\n)]"), text(&["x", "y"]));
    assert_eq!(arguments("#[a(0, 12, '')]"), text(&["0", "12", ""]));
    assert_eq!(arguments("#[a('say \"hé\"', \"it's\")]"), text(&["say \"hé\"", "it's"]));
    assert_eq!(arguments("#[a('(], )', '$ # @')]"), text(&["(], )", "$ # @"]));

    assert_eq!(arguments("#[a('$[0]_id', '$[1]$[x]')]"), vec![
        ExpressionAST::Action(vec![ExpressionAST::LocalCall { name: "0", span: 5..9 }, ExpressionAST::Text("_id")]),
        ExpressionAST::Action(vec![
            ExpressionAST::LocalCall { name: "1", span: 16..20 }, ExpressionAST::LocalCall { name: "x", span: 20..24 }
        ])
    ]);
    assert_eq!(arguments("#[a('#[b('é')]', @[0])]"), vec![
        ExpressionAST::Action(vec![ExpressionAST::ActionCall { name: "b", arguments: text(&["é"]), span: 5..8 }]),
        ExpressionAST::ArgumentCall { name: "0", span: 18..22 }
    ]);
}

#[test]
fn incomplete_arguments_are_errors() {
    let inputs = [
        "#[a(", "#[a(]", "#[a()", "#[a(x)]", "#[a($[0]$[1])]", "#[a('x' 'y')]", "#[a('x',)]", "#[a(,)]", "#[a('x)]", "#[a(\"x')]",
        "#[a('x') ]", "#[a ('x')]", "#[a(-1)]", "#[a(1x)]", "#[a('it's')]", "#[a($[0)]", "#[a('#[b')]", "x #[a(#[b(y)])]"
    ];

    for input in inputs {
        let error = syntax::parser::parse_expression(input).unwrap_err();

        assert!(error.to_string().contains("is not a complete call"), "{input}: {error}");
    }

    let error = syntax::parser::parse_expression("é #[a('x' 'y')]").unwrap_err();
    assert_eq!((error.to_string().split(':').next().unwrap(), error.span), ("'#[a(' is not a complete call", 3..7));

    let spans = |input| syntax::parser::parse_tolerant(input).unwrap().errors().into_iter().map(|error| error.span).collect::<Vec<_>>();
    assert_eq!(spans("#[a(x)] #[b('y')] $[0 #[c"), vec![0..4, 18..21, 22..25]);
    assert_eq!(spans("#[a(#[b(y)])]"), vec![0..4, 4..8]);
    assert!(spans("#[a('#[b('y')]')]").is_empty());
}

#[test]
fn deep_and_broken_calls_stay_fast() {
    let nested = |depth: usize| (
        "#[a('".repeat(depth) + &"#[b(\"".repeat(depth), "\")]".repeat(depth) + &"')]".repeat(depth)
    );

    let (opening, closing) = nested(4);
    let whole = format!("{opening}x{closing}");
    assert!(syntax::parser::parse_expression(&whole).is_ok());
    for end in 0..whole.len() {
        assert!(syntax::parser::parse_tolerant(&whole[..end]).is_ok());
    }

    let started = std::time::Instant::now();
    assert!(syntax::parser::parse_expression(&nested(60).0).is_err());
    assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
}

#[test]
fn any_text_is_parsed() {
    let pieces = ["#[", "$[", "@[", "a", "0", "]", "(", ")", "'", "\"", ",", " ", "é", "\n", "-", "#", "#[a(", ")]", "$[0]"];
    let mut state = 7u64;

    for _ in 0..20_000 {
        let mut input = String::new();
        for _ in 0..12 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            input += pieces[(state >> 33) as usize % pieces.len()];
        }

        let parsed = syntax::parser::parse_tolerant(&input).unwrap();
        for error in parsed.errors() {
            assert!(input.get(error.span).is_some(), "{input}");
        }
        assert_eq!(parsed.errors().is_empty(), syntax::parser::parse_expression(&input).is_ok());
    }
}
