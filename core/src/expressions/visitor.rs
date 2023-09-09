// Standard Uses

// Crate Uses
use crate::expressions::syntax::{ExpressionAST, parser};

// External Uses
use eyre::Result;


#[allow(unused)]
pub trait ExpressionVisitor {
    fn visit_expression(&mut self, name: &str, arguments: &Vec<ExpressionAST>);
    fn visit_arguments(&mut self, arguments: &Vec<ExpressionAST>);
    fn visit_text(&mut self, text: &str);
    fn visit_local_call(&mut self, variable: &str);
    fn visit_argument_call(&mut self, variable: &str);

    fn built(&self) -> String;
}

pub fn navigate_expression<'a>(
    visitor: &'a mut dyn ExpressionVisitor, input: &'a str
) -> Result<String> {
    let expr = parser::parse_expression(input)?;

    visit_expression(visitor, &expr);

    Ok(visitor.built())
}

fn visit_expression<'a>(
    visitor: &'a mut dyn ExpressionVisitor, expr: &ExpressionAST
) {
    match expr {
        ExpressionAST::Action(actions) => {
            actions.iter().for_each(|a| visit_expression(visitor, a))
        }
        ExpressionAST::ActionCall((name, args)) => {
            visitor.visit_expression(name, args)
        }
        ExpressionAST::LocalCall(var) => {
            visitor.visit_local_call(var);
        }
        ExpressionAST::ArgumentCall(var) => {
            visitor.visit_argument_call(var)
        }
        ExpressionAST::Text(text) => {
            visitor.visit_text(text);
        }
    }
}

