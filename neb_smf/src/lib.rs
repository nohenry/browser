#![feature(trait_upcasting)]

use std::collections::HashMap;

use ast::{Statement, StyleStatement};
use lexer::Lexer;
use log::{Log, SetLoggerError};
use neb_util::{
    format::{NodeDisplay, TreeDisplay},
    Rf,
};
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
use token::{SpannedToken, Token};

pub async fn parse_str(input: String) -> (Module, Vec<ParseError>) {
    let mut lexer = Lexer {};
    let tokens = lexer.lex(&input);

    let parser = Parser::new(tokens);
    let parsed = parser.parse().unwrap();

    let er = parser.get_errors().clone();

    let mods = Symbol::new_root();
    let mut md = ModuleDescender::new(mods.clone())
        .with_on_statement(|st, ud| {
            match st {
                Statement::Element { token, .. } | Statement::Style { token, .. } => {
                    let cd = if let Some(SpannedToken(_, Token::Ident(i))) = token {
                        Symbol::insert(&ud, &i, SymbolKind::Node)
                    } else {
                        Symbol::insert(&ud, &"view", SymbolKind::Node)
                    };
                    return (cd, ud);
                }
                Statement::UseStatement { args, .. } => {
                    let res: Option<Vec<String>> = args
                        .iter_items()
                        .map(|a| match a {
                            SpannedToken(_, Token::Ident(i)) => Some(i.clone()),
                            _ => None,
                        })
                        .collect();
                    if let Some(res) = res {
                        let cd = Symbol::insert(&ud, &"use", SymbolKind::Use(res));
                        return (cd, ud);
                    }
                }
                _ => (),
            }
            (ud.clone(), ud)
        })
        .with_on_style_statement(move |st, ud| {
            match st {
                StyleStatement::Style {
                    body: _,
                    body_range: _,
                    token: Some(SpannedToken(_, Token::Ident(i))),
                } => {
                    let cd = Symbol::insert(&ud, &i, SymbolKind::Style(st.clone()));
                    // let cd = Symbol::insert(&ud, &i, SymbolKind::Style(st.clone()));
                    return (cd, ud);
                }
                _ => (),
            }
            (ud.clone(), ud)
        });

    md.descend(&parsed);

    println!("{}", mods.format());

    (
        Module {
            content: input.to_string(),
            stmts: parsed,
            symbol_tree: mods,
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
    pub symbol_tree: Rf<Symbol>,
}

impl Module {
    pub fn format(&self) -> String {
        self.stmts
            .iter()
            .map(|f| format!("{}\n", f.format()))
            .collect()
    }

    pub fn resolve_symbol(&self, symbol_name: &str) {

    }

    pub fn resolve_symbol_chain<'a>(
        &self,
        iter: impl Iterator<Item = &'a SpannedToken>,
    ) -> Option<Rf<Symbol>> {
        self.impl_resolve_from_iter(&self.symbol_tree, iter).ok()
    }

    pub fn iter_symbol<'a, F: FnMut(&SpannedToken, &Rf<Symbol>)>(
        &self,
        iter: impl Iterator<Item = &'a SpannedToken>,
        f: F,
    ) {
        self.impl_iter_symbol(&self.symbol_tree, iter, f);
    }

    fn impl_iter_symbol<'a, F: FnMut(&SpannedToken, &Rf<Symbol>)>(
        &self,
        last: &Rf<Symbol>,
        mut iter: impl Iterator<Item = &'a SpannedToken>,
        mut f: F,
    ) {
        if let Some(tok @ SpannedToken(_, Token::Ident(i))) = iter.next() {
            if let Some(s) = last.borrow().children.get(i) {
                f(tok, s);
                self.impl_iter_symbol(s, iter, f);
            }
        }
    }

    fn impl_resolve_from_iter<'a>(
        &self,
        last: &Rf<Symbol>,
        mut iter: impl Iterator<Item = &'a SpannedToken>,
    ) -> Result<Rf<Symbol>, bool> {
        if let Some(SpannedToken(_, Token::Ident(i))) = iter.next() {
            if let Some(s) = last.borrow().children.get(i) {
                match self.impl_resolve_from_iter(s, iter) {
                    Ok(n) => return Ok(n),
                    Err(true) if &s.borrow().name == i => return Ok(s.clone()),
                    _ => (),
                }
            }
        } else {
            return Err(true);
        }
        Err(false)
    }
}

pub enum SymbolKind {
    Node,
    // StyleNode,
    Style(StyleStatement),
    Use(Vec<String>),
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
            SymbolKind::Node => write!(f, "Node `{}`", self.name),
            SymbolKind::Style(_) => write!(f, "Style `{}`", self.name),
            SymbolKind::Use(_) => write!(f, "Use"),
        }
    }
}

impl TreeDisplay for Symbol {
    fn num_children(&self) -> usize {
        self.children.len()
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        let p = self.children.values().nth(index).unwrap(); //.map(|f| &*f.borrow())

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

        symb.borrow().children.insert(name.to_string(), new.clone());

        new
    }
}

#[derive(Default)]
pub struct ModuleDescender<U: Clone> {
    user_data: U,
    on_statement: Option<Box<dyn FnMut(&Statement, U) -> (U, U)>>,
    on_style_statement: Option<Box<dyn FnMut(&StyleStatement, U) -> (U, U)>>,
    // on_value: Option<Box<fn(statement: &Value)>>,
}

impl<U: Clone> ModuleDescender<U> {
    pub fn new(user_data: U) -> ModuleDescender<U> {
        ModuleDescender {
            user_data,
            on_statement: None,
            on_style_statement: None,
        }
    }

    pub fn with_on_statement(
        mut self,
        on_statement: impl FnMut(&Statement, U) -> (U, U) + 'static,
    ) -> ModuleDescender<U> {
        self.on_statement = Some(Box::new(on_statement));
        self
    }

    pub fn with_on_style_statement(
        mut self,
        on_style_statement: impl FnMut(&StyleStatement, U) -> (U, U) + 'static,
    ) -> ModuleDescender<U> {
        self.on_style_statement = Some(Box::new(on_style_statement));
        self
    }

    pub fn descend(&mut self, node: &Vec<Statement>) {
        for node in node {
            self.descend_statement(node)
        }
    }

    pub fn descend_style_statements(&mut self, node: &Vec<StyleStatement>) {
        for node in node {
            self.descend_style_statement(node)
        }
    }

    pub fn descend_style_statement(&mut self, node: &StyleStatement) {
        let sets = if let Some(on_style_statement) = &mut self.on_style_statement {
            Some(on_style_statement(node, self.user_data.clone()))
        } else {
            None
        };
        let sets = if let Some(sets) = sets {
            self.user_data = sets.0;
            Some(sets.1)
        } else {
            None
        };
        match node {
            StyleStatement::Style { body, .. } => self.descend_style_statements(body),
            _ => (),
        }
        if let Some(sets) = sets {
            self.user_data = sets;
        }
    }

    pub fn descend_statement(&mut self, node: &Statement) {
        let sets = if let Some(on_statement) = &mut self.on_statement {
            Some(on_statement(node, self.user_data.clone()))
        } else {
            None
        };
        let sets = if let Some(sets) = sets {
            self.user_data = sets.0;
            Some(sets.1)
        } else {
            None
        };
        match node {
            Statement::Element { body, .. } => self.descend(body),
            Statement::Style { body, .. } => self.descend_style_statements(body),
            Statement::UseStatement { .. } => (),
        }
        if let Some(sets) = sets {
            self.user_data = sets;
        }
    }
}
