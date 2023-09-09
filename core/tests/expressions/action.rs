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

    assert_eq!(expression, expected_result);
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
    
    assert_eq!(expression, expected_result);
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

    assert_eq!(expected_result, expression);
}

