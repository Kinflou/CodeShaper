// Standard Uses

// Crate Uses
use crate::shaping::operation::expressions::common::{ActionParser, Expression, Rule};

// External Uses
use anyhow::Result;
use pest::{Parser, iterators::Pair};


#[allow(unused)]
pub trait ExpressionVisitor {
    fn visit_expression(&mut self, expr: &Expression) {}
    fn visit_expression_name(&mut self, name: &Expression) {}
    fn visit_arguments(&mut self, arguments: &Expression) {}
    fn visit_text(&mut self, text: &Expression) {}

}


pub fn navigate_expression<'a>(visitor: &'a mut dyn ExpressionVisitor, input: &'a str) -> Result<Expression<'a>> {
    let expression = ActionParser::parse(Rule::action, input)?.next().unwrap();

    fn parse_expression<'a>(visitor: &'a mut dyn ExpressionVisitor, pair: Pair<'a, Rule>) -> Expression<'a> {
        match pair.as_rule() {
            Rule::action => Expression::Action(
                pair.into_inner().map(|pair| {
                    let item = parse_expression(visitor, pair);

                    item
                }).collect::<Vec<Expression>>(),
            ),
            Rule::expression => {
                let mut inner = pair.into_inner();
                let name = inner.next().unwrap().as_str();

                let arguments = inner.next();
                if arguments.is_none() {
                    let expr = Expression::Expr((name, Box::new(Expression::Arguments(vec![]))));
                    visitor.visit_expression(&expr);
                    return expr
                }

                let arguments = parse_expression(
                    visitor, arguments.unwrap()
                );

                let expr = Expression::Expr((name, Box::new(arguments)));

                visitor.visit_expression(&expr);
                expr
            },
            Rule::expression_name => {
                let expression_name = Expression::ExpressionName(pair.as_str());

                visitor.visit_expression_name(&expression_name);
                expression_name
            },
            Rule::arguments => {
                let arguments = Expression::Arguments (
                    pair.into_inner().map(|p| {
                        parse_expression(visitor, p)
                    }).collect()
                );

                visitor.visit_arguments(&arguments);
                arguments
            }
            Rule::text => {
                let text = Expression::Text(pair.as_str().clone());

                visitor.visit_text(&text);
                text
            }
            _ => {
                // println!("{:?}", rule);
                Expression::None
            }
        }
    }

    Ok(parse_expression(visitor,expression))
}

