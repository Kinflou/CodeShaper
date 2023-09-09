// Standard Uses

// Crate Uses
use crate::expressions::syntax::ExpressionAST;

// External Uses
use eyre::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;


#[allow(unused)]
#[derive(Parser)]
#[grammar = "src/expressions/syntax/expression.pest"]
pub struct ActionParser;

pub fn parse_expression(input: &str) -> Result<ExpressionAST> {
    let expression = ActionParser::parse(Rule::action, input)?.next().unwrap();

    Ok(parse_pair(expression))
}


fn parse_pair(pair: Pair<Rule>) -> ExpressionAST {
    match pair.as_rule() {
        Rule::action => ExpressionAST::Action(
            pair.into_inner().map(|pair| {
                let item = parse_pair(pair);

                item
            }).collect::<Vec<ExpressionAST>>(),
        ),
        Rule::text => {
            ExpressionAST::Text(pair.as_str())
        }
        Rule::action_call => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str();

            let mut arguments = vec![];
            for arg in inner.into_iter() {
                arguments.push(parse_pair(arg));
            }

            ExpressionAST::ActionCall((name, arguments))
        },
        Rule::argument => {
            let mut inner = pair.into_inner();
            parse_pair(inner.next().unwrap())
        }
        Rule::string => {
            let text = pair.into_inner().next().unwrap().as_str();
            ExpressionAST::Text(text)
        }
        Rule::local_call => {
            ExpressionAST::LocalCall(pair.as_str())
        }
        missing => unreachable!("Rule not implemented {:?}", missing)
    }
}


