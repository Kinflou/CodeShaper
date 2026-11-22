// Standard Uses

// Crate Uses
use codeshaper_core::expressions::syntax;
use codeshaper_core::expressions::syntax::ExpressionAST;

// External Uses


#[allow(unused)]
#[test]
fn parse_simple_expression() {
    let some_expression = "#[some_expr](\"some\", \"value\")";

    let expression = syntax::parser::parse_expression(some_expression).unwrap();

    let expected_result = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall(("some_expr", vec![
            ExpressionAST::Text("some"),
            ExpressionAST::Text("value")
        ])),
    ]);

    pretty_assertions::assert_eq!(expected_result, expression);
}


#[allow(unused)]
#[test]
fn parse_nested_expression() {
    let nexted_expression = "#[parent_expr](\"value_1\", #[child_expr], \"value_3\")";

    let expression = syntax::parser::parse_expression(nexted_expression).unwrap();

    let expected_result = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall(("parent_expr", vec![
            ExpressionAST::Text("value_1"),
            ExpressionAST::ActionCall(("child_expr", vec![])),
            ExpressionAST::Text("value_3")
        ])),
    ]);

    pretty_assertions::assert_eq!(expression, expected_result);
}


#[allow(unused)]
#[test]
fn parse_multiple_nested_expression() {
    let nexted_expression = "#[parent_expr](\"value_1\", #[child_expr](#[child_child_expr]), \"value_3\")";

    let expression = syntax::parser::parse_expression(nexted_expression).unwrap();

    let expected_result = ExpressionAST::Action(vec![
        ExpressionAST::ActionCall(("parent_expr", vec![
                ExpressionAST::Text("value_1"),
                ExpressionAST::ActionCall(
                    ("child_expr", vec![
                        ExpressionAST::ActionCall(("child_child_expr", vec![])),
                    ])
                ),
                ExpressionAST::Text("value_3"),
            ])
        ),
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
