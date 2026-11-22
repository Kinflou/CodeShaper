// Standard Uses

// Crate Uses
use crate::expressions::syntax::ExpressionAST;

// External Uses
use miette::{bail, IntoDiagnostic, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;


#[allow(unused)]
#[derive(Parser)]
#[grammar = "src/expressions/syntax/expression.pest"]
pub struct ActionParser;

pub fn parse_expression(input: &str) -> Result<ExpressionAST<'_>> {
    let expression = ActionParser::parse(Rule::action, input).into_diagnostic()?.next().unwrap();

    parse_pair(expression)
}


fn parse_pair(pair: Pair<Rule>) -> Result<ExpressionAST> {
    let expression = match pair.as_rule() {
        Rule::action => ExpressionAST::Action(
            pair.into_inner()
                .filter(|pair| pair.as_rule() != Rule::EOI)
                .map(parse_pair)
                .collect::<Result<Vec<ExpressionAST>>>()?
        ),
        Rule::text => ExpressionAST::Text(pair.as_str()),
        Rule::unclosed => bail!("'{}' is not a complete call: expected a name followed by ']'", pair.as_str()),
        Rule::action_call => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str();
            let arguments = inner.map(parse_pair).collect::<Result<Vec<ExpressionAST>>>()?;

            ExpressionAST::ActionCall((name, arguments))
        }
        Rule::argument => parse_pair(pair.into_inner().next().unwrap())?,
        Rule::string => ExpressionAST::Text(pair.into_inner().next().unwrap().as_str()),
        Rule::local_call => ExpressionAST::LocalCall(pair.into_inner().next().unwrap().as_str()),
        Rule::argument_call => ExpressionAST::ArgumentCall(pair.into_inner().next().unwrap().as_str()),
        missing => unreachable!("Rule not implemented {:?}", missing)
    };

    Ok(expression)
}
