// Standard Uses

// Crate Uses

// External Uses


#[allow(unused)]
#[test]
fn parse_simple_expression() {
    let some_expression = "#[some_expr](some, value)";
}


#[allow(unused)]
#[test]
fn parse_nested_expression() {
    let nexted_expression = "#[parent_expr](value_1, #[child_expr], value_3)";
}


#[allow(unused)]
#[test]
fn parse_multiple_nested_expression() {
    let nexted_expression = "\
    #[parent_expr](value_1, #[child_expr](#[child_child_expr]), value_3)";
}

