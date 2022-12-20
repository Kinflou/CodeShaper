// Standard Uses

// Crate Uses
use codeshaper_general::shaping::operation::expressions::common;
use codeshaper_general::shaping::operation::expressions::common::Expression;

// External Uses


#[allow(unused)]
#[test]
fn parse_simple_expression() {
    let some_expression = "#[some_expr](\"some\", \"value\")";

    let expression = common::parse_expression(some_expression).unwrap();

    let expected_result = Expression::Action(vec![
        Expression::Expr(("some_expr", Box::new(Expression::Arguments(
            vec![
                Expression::Text("some"),
                Expression::Text("value")
            ])
        ))),
        Expression::None
    ]);

    assert_eq!(expression, expected_result);
}


#[allow(unused)]
#[test]
fn parse_nested_expression() {
    let nexted_expression = "#[parent_expr](\"value_1\", #[child_expr], \"value_3\")";

    let expression = common::parse_expression(nexted_expression).unwrap();

    let expected_result = Expression::Action(vec![
        Expression::Expr(("parent_expr", Box::new(Expression::Arguments(
            vec![
                Expression::Text("value_1"),
                Expression::Expr(("child_expr", Box::from(Expression::Arguments(vec![])))),
                    Expression::Text("value_3")
            ])
        ))),
        Expression::None
    ]);
    
    assert_eq!(expression, expected_result);
}


#[allow(unused)]
#[test]
fn parse_multiple_nested_expression() {
    let nexted_expression = "\
    #[parent_expr](\"value_1\", #[child_expr](#[child_child_expr]), \"value_3\")";

    let expression = common::parse_expression(nexted_expression).unwrap();

    let expected_result = Expression::Action(vec![
        Expression::Expr(("parent_expr", Box::new(Expression::Arguments(
            vec![
                Expression::Text("value_1"),
                Expression::Expr(
                    ("child_expr", Box::new(Expression::Arguments(
                        vec![
                            Expression::Expr(("child_child_expr", Box::new(Expression::Arguments(vec![])))),
                        ])
                    ))
                ),
                Expression::Text("value_3"),
            ])
        ))),
        Expression::None
    ]);

    assert_eq!(expression, expected_result);
}

