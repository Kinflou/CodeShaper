// Relative Modules
pub mod parser;


#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ExpressionAST<'a> {
    Action(Vec<ExpressionAST<'a>>),
    Text(&'a str),
    ActionCall((&'a str, Vec<ExpressionAST<'a>>)),
    LocalCall(&'a str),
    ArgumentCall(&'a str),
}
