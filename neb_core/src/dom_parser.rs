use std::{
    io::{BufReader, Read},
};

use neb_errors::DocumentError;
use neb_graphics::{drawing_context::DrawingContext, vello::kurbo::Rect};
use neb_smf::{Module, Symbol, SymbolKind};
use neb_util::{format::TreeDisplay, Rf};

use crate::{
    // svg::{self, PicoSvg},
    node::{Node, NodeType},
};

pub fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

pub struct Document {
    errors: Vec<DocumentError>,

    body_root: Rf<Node>,

    styles: Option<Rf<Symbol>>,
}

impl Document {
    pub fn get_errors(&self) -> &[DocumentError] {
        &self.errors
    }

    pub fn get_body(&self) -> &Rf<Node> {
        &self.body_root
    }

    pub fn get_styles(&self) -> Option<&Rf<Symbol>> {
        self.styles.as_ref()
    }
}

impl Document {
    pub fn draw(&self, dctx: &mut DrawingContext) {
        let body = self.body_root.borrow();
        body.draw(dctx, self);
    }

    pub fn layout(&self, width: f64, height: f64) {
        let body = self.body_root.borrow();
        body.get_element().layout(
            &body,
            Rect::from_origin_size((0.0, 0.0), (width, height)),
            0,
            self,
        );
    }
}

pub fn parse_from_stream<R>(mut stream: BufReader<R>) -> Document
where
    R: Read,
{
    let mut input = String::new();
    let _ = stream.read_to_string(&mut input).unwrap();

    let (mods, _) = Module::parse_str(&input);

    let root = Rf::new(Node::new_root(NodeType::Root));

    let mod_tree = mods.symbol_tree.borrow();

    for symbol in mod_tree.children.values() {
        let Some(p) = build_nodes(root.clone(), symbol) else {
            continue;
        };
        let mut root = root.borrow_mut();
        root.add_child(p);
    }

    println!("{}", root.borrow().format());

    Document {
        errors: Vec::new(),
        body_root: root,
        styles: None,
    }
}

fn build_nodes(parent: Rf<Node>, symbol: &Rf<Symbol>) -> Option<Rf<Node>> {
    let symbol = symbol.borrow();
    match &symbol.kind {
        SymbolKind::Node { args } => {
            let ty = if symbol.name == "view" {
                NodeType::View { args: args.clone() }
            } else if symbol.name == "style" {
                NodeType::StyleBlock
            } else {
                NodeType::Setup
            };
            let node = Rf::new(Node::new(ty, parent));

            for (_name, val) in symbol.children.iter() {
                let Some(child) = build_nodes(node.clone(), val) else {
                    continue;
                };

                let mut node = node.borrow_mut();
                node.add_child(child);
            }

            Some(node)
        }
        SymbolKind::Style { properties } => Some(Rf::new(Node::new(
            NodeType::Style {
                name: symbol.name.clone(),
                properties: properties.clone(),
            },
            parent,
        ))),
        SymbolKind::Text(s) => Some(Rf::new(Node::new(NodeType::Text(s.clone()), parent))),
        _ => None,
    }
}
