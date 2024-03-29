#![feature(trait_upcasting)]

use std::collections::HashMap;

use ast::{Statement, StyleStatement, Value};
use lexer::Lexer;
use linked_hash_map::LinkedHashMap;
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

impl Module {
    pub fn parse_str(input: &str) -> (Module, Vec<ParseError>) {
        let mut lexer = Lexer {};
        let tokens = lexer.lex(&input);
        for tok in &tokens {
            println!("{:?}", tok);
        }

        let parser = Parser::new(tokens);
        let parsed = parser.parse().unwrap();
        for p in &parsed {
            println!("{}", p.format());
        }

        let er = parser.get_errors().clone();

        let mods = Symbol::new_root();
        let md = ModuleDescender::new(mods.clone())
            .with_on_statement(|st, ud| {
                match st {
                    Statement::Element {
                        token, arguments, ..
                    } => {
                        let args = if let Some(args) = arguments {
                            let vals = args.iter_items().filter_map(|arg| {
                                if let (Some(SpannedToken(_, Token::Ident(name))), Some(value)) =
                                    (&arg.name, &arg.value)
                                {
                                    Some((name.clone(), value.clone()))
                                } else {
                                    None
                                }
                            });
                            HashMap::from_iter(vals)
                        } else {
                            HashMap::new()
                        };
                        let cd = if let Some(SpannedToken(_, Token::Ident(i))) = token {
                            match i.as_str() {
                                "setup" | "style" => {
                                    Some(Symbol::insert(&ud, i, SymbolKind::Node { args }))
                                }
                                _ => Symbol::insert_unnamed(&ud, i, SymbolKind::Node { args }),
                            }
                        } else {
                            Symbol::insert_unnamed(&ud, "view", SymbolKind::Node { args })
                        };
                        if let Some(cd) = cd {
                            return (cd, ud);
                        }
                    }
                    Statement::Text(SpannedToken(_, Token::Text(i))) => {
                        let cd = Symbol::insert_unnamed(&ud, "text", SymbolKind::Text(i.clone()));
                        if let Some(cd) = cd {
                            return (cd, ud);
                        } else {
                            return (ud.clone(), ud);
                        }
                    }
                    Statement::Style { token, .. } => {
                        let cd = if let Some(SpannedToken(_, Token::Ident(i))) = token {
                            Symbol::insert(
                                &ud,
                                &i,
                                SymbolKind::Node {
                                    args: HashMap::new(),
                                },
                            )
                        } else {
                            Symbol::insert(
                                &ud,
                                &"view",
                                SymbolKind::Node {
                                    args: HashMap::new(),
                                },
                            )
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
                        let cd = Symbol::insert(
                            &ud,
                            &i,
                            SymbolKind::Style {
                                properties: HashMap::from_iter(st.style_elements()),
                            },
                        );
                        return (cd, ud);
                    }
                    _ => (),
                }
                (ud.clone(), ud)
            });

        md.descend(&parsed);

        {
            // let mods = mods.borrow_mut();
            Symbol::insert(
                &mods,
                "rgb",
                SymbolKind::Function {
                    args: vec![Type::Integer, Type::Integer, Type::Integer],
                    return_type: Type::Tuple(vec![Type::Integer, Type::Integer, Type::Integer]),
                    func: Box::new(|vals| Some(Value::Tuple(vals))),
                },
            );

            Symbol::insert(
                &mods,
                "rgba",
                SymbolKind::Function {
                    args: vec![Type::Integer, Type::Integer, Type::Integer, Type::Integer],
                    return_type: Type::Tuple(vec![
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                    ]),
                    func: Box::new(|vals| Some(Value::Tuple(vals))),
                },
            );

            Symbol::insert(
                &mods,
                "rect",
                SymbolKind::Function {
                    args: vec![Type::Integer, Type::Integer, Type::Integer, Type::Integer],
                    return_type: Type::Tuple(vec![
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                    ]),
                    func: Box::new(|vals| Some(Value::Tuple(vals))),
                },
            );

            Symbol::insert(
                &mods,
                "rect_xy",
                SymbolKind::Function {
                    args: vec![Type::Integer, Type::Integer],
                    return_type: Type::Tuple(vec![
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                    ]),
                    func: Box::new(|vals| {
                        Some(Value::Tuple(vec![
                            vals[0].clone(),
                            vals[1].clone(),
                            vals[0].clone(),
                            vals[1].clone(),
                        ]))
                    }),
                },
            );

            Symbol::insert(
                &mods,
                "rect_all",
                SymbolKind::Function {
                    args: vec![Type::Integer],
                    return_type: Type::Tuple(vec![
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                        Type::Integer,
                    ]),
                    func: Box::new(|vals| {
                        Some(Value::Tuple(vec![
                            vals[0].clone(),
                            vals[0].clone(),
                            vals[0].clone(),
                            vals[0].clone(),
                        ]))
                    }),
                },
            );
        }

        println!("Mods {}", mods.format());

        (
            Module {
                content: input.to_string(),
                stmts: parsed,
                symbol_tree: mods,
            },
            er,
        )
    }
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

    pub fn resolve_symbol_in_scope<'a>(
        &self,
        symbol: &str,
        scope: impl Iterator<Item = &'a String>,
    ) -> Option<Rf<Symbol>> {
        let Some(sym) = self.resolve_symbol_chain_string(scope) else {
            return None
        };
        self.impl_resolve_symbol_in_scope(symbol, &sym)
    }

    pub fn impl_resolve_symbol_in_scope<'a>(
        &self,
        symbol: &str,
        node: &Rf<Symbol>,
    ) -> Option<Rf<Symbol>> {
        let nodev = node.borrow();
        match nodev.kind {
            SymbolKind::Style { .. } if nodev.name == symbol => return Some(node.clone()),
            SymbolKind::Use(_) => return None,
            _ => (),
        }
        if let Some(child) = nodev.children.get(symbol) {
            Some(child.clone())
        } else {
            for (_, child) in &nodev.children {
                let child = child.borrow();
                if let SymbolKind::Use(scp) = &child.kind {
                    return self.resolve_symbol_in_scope(symbol, scp.iter());
                }
            }
            None
        }
    }

    pub fn resolve_symbol(&self, node: &Rf<Symbol>, symbol_name: &str) -> Option<Rf<Symbol>> {
        if let Some(node) = self.impl_resolve_symbol_in_scope(symbol_name, node) {
            Some(node)
        } else {
            let Some(parent) = ({ &node.borrow().parent }) else {
                    return None
                };

            self.resolve_symbol(parent, symbol_name)
        }
    }

    pub fn resolve_symbol_chain_indicies<'a>(
        &self,
        iter: impl Iterator<Item = &'a usize>,
    ) -> Option<Rf<Symbol>> {
        self.impl_resolve_symbol_chain_indicies(&self.symbol_tree, iter)
            .ok()
    }

    fn impl_resolve_symbol_chain_indicies<'a>(
        &self,
        last: &Rf<Symbol>,
        mut iter: impl Iterator<Item = &'a usize>,
    ) -> Result<Rf<Symbol>, bool> {
        if let Some(index) = iter.next() {
            if let Some(s) = last.borrow().children.values().nth(*index) {
                match self.impl_resolve_symbol_chain_indicies(s, iter) {
                    Ok(n) => return Ok(n),
                    Err(true) => return Ok(s.clone()),
                    _ => (),
                }
            }
        } else {
            return Err(true);
        }
        Err(false)
    }

    pub fn resolve_symbol_chain<'a>(
        &self,
        iter: impl Iterator<Item = &'a SpannedToken>,
    ) -> Option<Rf<Symbol>> {
        self.impl_resolve_from_iter(&self.symbol_tree, iter).ok()
    }

    pub fn resolve_symbol_chain_string<'a>(
        &self,
        iter: impl Iterator<Item = &'a String>,
    ) -> Option<Rf<Symbol>> {
        self.impl_resolve_from_iter_string(&self.symbol_tree, iter)
            .ok()
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
                    Err(true) => return Ok(s.clone()),
                    _ => (),
                }
            }
        } else {
            return Err(true);
        }
        Err(false)
    }

    fn impl_resolve_from_iter_string<'a>(
        &self,
        last: &Rf<Symbol>,
        mut iter: impl Iterator<Item = &'a String>,
    ) -> Result<Rf<Symbol>, bool> {
        if let Some(i) = iter.next() {
            if let Some(s) = last.borrow().children.get(i) {
                match self.impl_resolve_from_iter_string(s, iter) {
                    Ok(n) => return Ok(n),
                    Err(true) => return Ok(s.clone()),
                    _ => (),
                }
            }
        } else {
            return Err(true);
        }
        Err(false)
    }
}

pub enum Type {
    None,
    Float,
    Integer,
    Ident(String),
    Tuple(Vec<Type>),
}

impl Type {
    pub fn value_is_type(&self, value: &Value) -> bool {
        match (self, value) {
            (Type::Float, Value::Float(_, _, _)) => true,
            (Type::Integer, Value::Integer(_, _, _)) => true,
            _ => false,
        }
    }
}

pub enum SymbolKind {
    Text(String),
    Node {
        args: HashMap<String, Value>,
    },
    Function {
        args: Vec<Type>,
        return_type: Type,
        func: Box<dyn Fn(Vec<Value>) -> Option<Value> + Send + Sync>,
    },
    Style {
        properties: HashMap<String, Value>,
    },
    Use(Vec<String>),
    Root,
}

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub parent: Option<Rf<Symbol>>,
    pub children: LinkedHashMap<String, Rf<Symbol>>,
}

impl NodeDisplay for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.kind {
            SymbolKind::Root => f.write_str("Root"),
            SymbolKind::Function { .. } => write!(f, "Function `{}`", self.name),
            SymbolKind::Text(s) => write!(f, "Text `{}`", s),
            SymbolKind::Node { .. } => write!(f, "Node `{}`", self.name),
            SymbolKind::Style { .. } => write!(f, "Style `{}`", self.name),
            SymbolKind::Use(_) => write!(f, "Use"),
        }
    }
}

impl TreeDisplay for Symbol {
    fn num_children(&self) -> usize {
        self.children.len()
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        None
    }

    fn child_at_bx<'a>(&'a self, index: usize) -> Box<dyn TreeDisplay + 'a> {
        let p = self.children.values().nth(index).unwrap().borrow(); //.map(|f| &*f.borrow())

        Box::new(p)
    }
}

impl Symbol {
    pub fn new_root() -> Rf<Symbol> {
        Rf::new(Symbol {
            name: "root".to_string(),
            kind: SymbolKind::Root,
            parent: None,
            children: LinkedHashMap::new(),
        })
    }

    pub fn insert_unnamed(symb: &Rf<Symbol>, name: &str, kind: SymbolKind) -> Option<Rf<Symbol>> {
        let insert_index = {
            let symb = symb.borrow();

            // Find free index; max 128
            [0; 128]
                .into_iter()
                .enumerate()
                .map(|(i, _)| i)
                .find_map(|v| {
                    let val = format!("{}", v);
                    if symb.children.get(&val).is_none() {
                        Some(val)
                    } else {
                        None
                    }
                })
        };

        if let Some(insert_index) = insert_index {
            let new = Rf::new(Symbol {
                name: name.into(),
                kind,
                parent: Some(symb.clone()),
                children: LinkedHashMap::new(),
            });

            symb.borrow_mut().children.insert(insert_index, new.clone());

            Some(new)
        } else {
            None
        }
    }

    pub fn insert(symb: &Rf<Symbol>, name: &str, kind: SymbolKind) -> Rf<Symbol> {
        let new = Rf::new(Symbol {
            name: name.to_string(),
            kind,
            parent: Some(symb.clone()),
            children: LinkedHashMap::new(),
        });

        symb.borrow_mut()
            .children
            .insert(name.to_string(), new.clone());

        new
    }
}

#[derive(Default)]
pub struct ModuleDescender<U: Clone> {
    user_data: U,
    on_statement: Option<Box<dyn FnMut(&Statement, U) -> (U, U)>>,
    on_style_statement: Option<Box<dyn FnMut(&StyleStatement, U) -> (U, U)>>,
    on_value: Option<Box<dyn FnMut(Option<&SpannedToken>, &Value, U) -> U>>,
    // on_value: Option<Box<fn(statement: &Value)>>,
}

impl<U: Clone> ModuleDescender<U> {
    pub fn new(user_data: U) -> ModuleDescender<U> {
        ModuleDescender {
            user_data,
            on_statement: None,
            on_style_statement: None,
            on_value: None,
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

    pub fn with_on_value(
        mut self,
        on_value: impl FnMut(Option<&SpannedToken>, &Value, U) -> U + 'static,
    ) -> ModuleDescender<U> {
        self.on_value = Some(Box::new(on_value));
        self
    }

    pub fn descend(mut self, node: &Vec<Statement>) -> U {
        for node in node {
            self.descend_statement(node)
        }
        self.user_data
    }

    pub fn descend_style_statements(&mut self, node: &Vec<StyleStatement>) {
        for node in node {
            self.descend_style_statement(node)
        }
    }

    pub fn descend_value(&mut self, key: Option<&SpannedToken>, node: &Value) {
        if let Some(on_value) = &mut self.on_value {
            self.user_data = on_value(key, node, self.user_data.clone())
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
            StyleStatement::StyleElement {
                key,
                value: Some(node),
                ..
            } => self.descend_value(key.as_ref(), node),
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
            Statement::Element { body, .. } => body.iter().for_each(|s| self.descend_statement(s)),
            Statement::Style { body, .. } => self.descend_style_statements(body),
            Statement::UseStatement { .. } => (),
            Statement::Text(_) => (),
        }
        if let Some(sets) = sets {
            self.user_data = sets;
        }
    }
}

#[derive(Default)]
pub struct MutModuleDescender<U: Clone> {
    callback_first: bool,
    user_data: U,
    on_statement: Option<Box<dyn FnMut(&mut Statement, U) -> (U, U)>>,
    on_style_statement: Option<Box<dyn FnMut(&mut StyleStatement, U) -> (U, U)>>,
    on_value: Option<Box<dyn FnMut(Option<&mut SpannedToken>, &mut Value, U) -> U>>,
    // on_value: Option<Box<fn(statement: &Value)>>,
}

impl<U: Clone> MutModuleDescender<U> {
    pub fn new(user_data: U) -> MutModuleDescender<U> {
        MutModuleDescender {
            callback_first: true,
            user_data,
            on_statement: None,
            on_style_statement: None,
            on_value: None,
        }
    }

    pub fn with_on_statement(
        mut self,
        on_statement: impl FnMut(&mut Statement, U) -> (U, U) + 'static,
    ) -> MutModuleDescender<U> {
        self.on_statement = Some(Box::new(on_statement));
        self
    }

    pub fn with_on_style_statement(
        mut self,
        on_style_statement: impl FnMut(&mut StyleStatement, U) -> (U, U) + 'static,
    ) -> MutModuleDescender<U> {
        self.on_style_statement = Some(Box::new(on_style_statement));
        self
    }

    pub fn with_on_value(
        mut self,
        on_value: impl FnMut(Option<&mut SpannedToken>, &mut Value, U) -> U + 'static,
    ) -> MutModuleDescender<U> {
        self.on_value = Some(Box::new(on_value));
        self
    }

    pub fn with_callback_first(mut self, callback_first: bool) -> MutModuleDescender<U> {
        self.callback_first = callback_first;
        self
    }

    pub fn descend(mut self, node: &mut Vec<Statement>) -> U {
        for node in node {
            self.descend_statement(node)
        }
        self.user_data
    }

    pub fn descend_style_statements(&mut self, node: &mut Vec<StyleStatement>) {
        for node in node {
            self.descend_style_statement(node)
        }
    }

    pub fn descend_value(&mut self, key: Option<&mut SpannedToken>, node: &mut Value) {
        if let Some(on_value) = &mut self.on_value {
            self.user_data = on_value(key, node, self.user_data.clone())
        }
    }

    pub fn descend_style_statement(&mut self, node: &mut StyleStatement) {
        if self.callback_first {
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
                StyleStatement::StyleElement {
                    key,
                    value: Some(node),
                    ..
                } => self.descend_value(key.as_mut(), node),
                _ => (),
            }
            if let Some(sets) = sets {
                self.user_data = sets;
            }
        } else {
            match node {
                StyleStatement::Style { body, .. } => self.descend_style_statements(body),
                StyleStatement::StyleElement {
                    key,
                    value: Some(node),
                    ..
                } => self.descend_value(key.as_mut(), node),
                _ => (),
            }
            if let Some(on_style_statement) = &mut self.on_style_statement {
                self.user_data = on_style_statement(node, self.user_data.clone()).1
            }
        }
    }

    pub fn descend_statement(&mut self, node: &mut Statement) {
        if self.callback_first {
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
                Statement::Element { body, .. } => {
                    body.iter_mut().for_each(|s| self.descend_statement(s))
                }
                Statement::Style { body, .. } => self.descend_style_statements(body),
                Statement::UseStatement { .. } => (),
                Statement::Text(_) => (),
            }
            if let Some(sets) = sets {
                self.user_data = sets;
            }
        } else {
            match node {
                Statement::Element { body, .. } => {
                    body.iter_mut().for_each(|s| self.descend_statement(s))
                }
                Statement::Style { body, .. } => self.descend_style_statements(body),
                Statement::UseStatement { .. } => (),
                Statement::Text(_) => (),
            }
            if let Some(on_statement) = &mut self.on_statement {
                self.user_data = on_statement(node, self.user_data.clone()).1
            }
        }
    }
}
