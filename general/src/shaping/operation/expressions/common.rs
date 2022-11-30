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

#[allow(unused)]
pub enum Expression<'a> {
    ExpressionName(&'a str),
    Expr((&'a str, Vec<(Expression<'a>)>)),
    Arguments(Vec<Expression<'a>>),
    Text(&'a str)
}

#[allow(unused)]
fn parse_expression(input: &str) -> Result<Expression> {
    let expression = ActionParser::parse(Rule::action, &input)?.next().unwrap();

    fn parse_expression(pair: Pair<Rule>) -> Expression {
        match pair.as_rule() {
            Rule::expression_name => {
                let name = pair.into_inner().next().unwrap().as_str();

                Expression::ExpressionName(name)
            },
            Rule::expression => {
                let mut inner = pair.into_inner();
                let name = inner.next().unwrap().as_str();

                let arguments = inner.map(|p| {
                    parse_expression(p.into_inner().next().unwrap())
                }).collect();

                Expression::Expr((name, arguments))
            },
            Rule::arguments => {
                Expression::Arguments (
                    pair.into_inner().map(parse_expression).collect()
                )
            }
            Rule::text | Rule::expression_name => {
                Expression::Text(pair.into_inner().next().unwrap().as_str())
            }

            Rule::action
            | Rule::EOI
            | Rule::value
            | Rule::char
            | Rule::WHITESPACE => unreachable!(),
        }
    }

    Ok(parse_expression(expression))
}


