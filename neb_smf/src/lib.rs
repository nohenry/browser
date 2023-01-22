#![feature(trait_upcasting)]

use ast::Statement;
use format::TreeDisplay;
use lexer::Lexer;
use log::{Log, SetLoggerError};
use parser::Parser;

pub mod ast;
pub mod error;
pub mod format;
pub mod lexer;
pub mod logger;
pub mod parser;
pub mod style_parser;
pub mod token;

use error::ParseError;
pub use pollster;

pub async fn parse_str(input: String) -> (Module, Vec<ParseError>) {
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

pub fn set_logger(logger: Box<dyn Log>) -> Result<(), SetLoggerError> {
    log::set_boxed_logger(logger)
}

pub struct Module {
    pub content: String,
    pub stmts: Vec<Statement>,
}

impl Module {
    pub fn format(&self) -> String {
        self.stmts
            .iter()
            .map(|f| format!("{}\n", f.format()))
            .collect()
    }
}
