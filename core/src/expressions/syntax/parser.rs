// Standard Uses

// Crate Uses
use crate::expressions::syntax::{ExpressionAST, ExpressionError};

// External Uses
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;


#[allow(unused)]
#[derive(Parser)]
#[grammar = "src/expressions/syntax/expression.pest"]
pub struct ActionParser;

pub fn parse_expression(input: &str) -> Result<ExpressionAST<'_>, ExpressionError> {
    let expression = parse_tolerant(input)?;

    match expression.errors().into_iter().next() {
        Some(error) => Err(error),
        None => Ok(expression)
    }
}

pub fn parse_tolerant(input: &str) -> Result<ExpressionAST<'_>, ExpressionError> {
    let expression = ActionParser::parse(Rule::action, input)
        .map_err(|error| ExpressionError { message: error.to_string(), span: 0..input.len() })?
        .next()
        .unwrap();

    Ok(parse_pair(expression))
}


fn parse_pair<'a>(pair: Pair<'a, Rule>) -> ExpressionAST<'a> {
    let start = pair.as_span().start();
    let head = |name: &str| start..start + name.len() + 3;
    let parts = |pair: Pair<'a, Rule>| pair.into_inner()
        .filter(|pair| pair.as_rule() != Rule::EOI)
        .map(parse_pair)
        .collect::<Vec<_>>();

    match pair.as_rule() {
        Rule::action => ExpressionAST::Action(parts(pair)),
        Rule::text | Rule::single_text | Rule::double_text | Rule::number => ExpressionAST::Text(pair.as_str()),
        Rule::unclosed => {
            let rest = &pair.as_span().get_input()[start..];
            let name = rest[2..].find(|character| !is_name(character)).unwrap_or(rest.len() - 2) + 2;
            let length = name + usize::from(rest.starts_with('#') && rest[name..].starts_with('('));

            ExpressionAST::Unclosed { text: &rest[..length], span: start..start + length }
        }
        Rule::action_call => {
            let end = pair.as_span().end();
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str();
            let arguments = inner.map(parse_pair).collect();
            let head = head(name);
            let span = if head.end == end { head } else { head.start..head.end - 1 };

            ExpressionAST::ActionCall { name, arguments, span }
        }
        Rule::single | Rule::double => {
            let parts = parts(pair);

            match parts.as_slice() {
                [] => ExpressionAST::Text(""),
                [ExpressionAST::Text(text)] => ExpressionAST::Text(text),
                _ => ExpressionAST::Action(parts)
            }
        }
        Rule::local_call => {
            let name = pair.into_inner().next().unwrap().as_str();

            ExpressionAST::LocalCall { name, span: head(name) }
        }
        Rule::argument_call => {
            let name = pair.into_inner().next().unwrap().as_str();

            ExpressionAST::ArgumentCall { name, span: head(name) }
        }
        missing => unreachable!("Rule not implemented {:?}", missing)
    }
}

fn is_name(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '!')
}
