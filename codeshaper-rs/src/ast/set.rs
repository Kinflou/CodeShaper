// Standard Uses

// Crate Uses

// External Uses
use antlr_rust::{InputStream, Lexer, Parser};
use antlr_rust::parser::ParserNodeType;
use antlr_rust::parser_rule_context::ParserRuleContext;


pub trait ASTSet<'input, T, Ctx> where T: {
    const NAME: &'static str;
    const ALIAS: &'static str;

    const LEXER: fn() -> Box<dyn Lexer<'input, TF=T, Node=T, Input=()>>;
    const PARSER: fn() -> Box<dyn Parser<'input, TF=T, Node=T>>;

    fn input_stream(&self) -> InputStream<&str>;

    fn get_root_context(&self) -> Box<dyn ParserNodeType>;

    fn get_context_text(&self, context: &str) -> String {
        // self.input_stream[]
        todo!()
    }
}

