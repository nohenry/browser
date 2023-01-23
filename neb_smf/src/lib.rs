#![feature(trait_upcasting)]

use std::collections::HashMap;

use ast::Statement;
use lexer::Lexer;
use log::{Log, SetLoggerError};
use neb_util::{Rf, format::{NodeDisplay, TreeDisplay}};
use parser::Parser;

pub mod ast;
pub mod error;
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

pub enum SymbolKind {
    Style,
    Root,
}

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub parent: Option<Rf<Symbol>>,
    pub children: HashMap<String, Rf<Symbol>>,
}

impl NodeDisplay for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.kind {
            SymbolKind::Root => f.write_str("Root"),
            SymbolKind::Style => write!(f, "Style `{}`", self.name),
        }
    }
}

impl TreeDisplay for Symbol {
    fn num_children(&self) -> usize {
        self.children.len()
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        let p = self.children.values().nth(index).unwrap();//.map(|f| &*f.borrow())

        Some(p)
    }
}

impl Symbol {
    pub fn new_root() -> Rf<Symbol> {
        Rf::new(Symbol {
            name: "root".to_string(),
            kind: SymbolKind::Root,
            parent: None,
            children: HashMap::new(),
        })
    }

    pub fn insert(symb: &Rf<Symbol>, name: &str, kind: SymbolKind) -> Rf<Symbol> {
        let new = Rf::new(Symbol {
            name: name.to_string(),
            kind,
            parent: Some(symb.clone()),
            children: HashMap::new(),
        });

        symb.borrow_mut()
            .children
            .insert(name.to_string(), new.clone());

        new
    }
}
