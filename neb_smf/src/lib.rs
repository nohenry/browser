use ast::Statement;
use lexer::Lexer;
use parser::Parser;

pub mod ast;
pub mod error;
pub mod format;
pub mod lexer;
pub mod parser;
pub mod token;

use error::ParseError;

pub fn parse_str(input: String) -> (Module, Vec<ParseError>) {
    let mut lexer = Lexer {};
    let tokens = lexer.lex(&input);

    let parser = Parser::new(tokens);
    let parsed = parser.parse().unwrap();

    let er = parser.get_errors().clone();

    (
        Module {
            content: input.to_string(),
            stmts: parsed,
        },
        er,
    )
}

pub struct Module {
    pub content: String,
    pub stmts: Vec<Statement>,
}
