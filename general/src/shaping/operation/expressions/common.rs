// Standard Uses

// Crate Uses

// External Uses
use anyhow::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;


#[allow(unused)]
#[derive(Parser)]
#[grammar = "src/shaping/operation/expressions/expression.pest"]
pub struct ActionParser;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Expression<'a> {
    Action(Vec<Expression<'a>>),
    ExpressionName(&'a str),
    Expr((&'a str, Box<Expression<'a>>)),
    Arguments(Vec<Expression<'a>>),
    Text(&'a str),
    None
}

pub fn parse_expression(input: &str) -> Result<Expression> {
    let expression = ActionParser::parse(Rule::action, input)?.next().unwrap();

    fn parse_expression(pair: Pair<Rule>) -> Expression {
        match pair.as_rule() {
            Rule::action => Expression::Action(
                pair.into_inner().map(|pair| {
                    let item = parse_expression(pair);

                    item
                }).collect::<Vec<Expression>>(),
            ),
            Rule::expression => {
                let mut inner = pair.into_inner();
                let name = inner.next().unwrap().as_str();

                let arguments = inner.next();
                if arguments.is_none() {
                    return Expression::Expr((name, Box::new(Expression::Arguments(vec![]))))
                }

                let arguments = parse_expression(arguments.unwrap());

                Expression::Expr((name, Box::new(arguments)))
            },
            Rule::expression_name => {
                Expression::ExpressionName(pair.as_str())
            },
            Rule::arguments => {
                Expression::Arguments (
                    pair.into_inner().map(parse_expression).collect()
                )
            }
            Rule::text => {
                let mut inner = pair.into_inner();

                Expression::Text(inner.next().unwrap().as_str())
            }
            _ => {
                // unreachable!("{:?}", other)
                Expression::None
            }
        }
    }

    Ok(parse_expression(expression))
}


