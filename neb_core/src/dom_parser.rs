use std::{
    collections::HashMap,
    fmt::Display,
    io::{BufReader, Read},
};

use xml::{reader::XmlEvent, EventReader};

use crate::{tree_display::TreeDisplay, Rf};

fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

pub struct Document {}

/// The node type is a specific type of element
/// The most common element is the `Div` which is for general use case
#[derive(Debug)]
pub enum NodeType {
    /// A general element type
    Div,

    Head,

    Body,

    Html,
}

impl NodeType {
    pub fn try_node(element: &str) -> Option<NodeType> {
        use NodeType::*;

        match element.to_lowercase().as_str() {
            "div" => Some(Div),
            "html" => Some(Html),
            "head" => Some(Head),
            "body" => Some(Body),
            _ => None,
        }
    }

    pub fn try_node_poss(element: Option<&str>) -> Option<NodeType> {
        match element {
            Some(node) => NodeType::try_node(node),
            _ => None,
        }
    }
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NodeType::*;

        match self {
            Div => write!(f, "div"),
            Head => write!(f, "head"),
            Body => write!(f, "body"),
            Html => write!(f, "html"),
        }
    }
}

/// A node that represents an element in the document tree
#[derive(Debug)]
pub struct Node {
    /// The specific type that this node represents
    ty: NodeType,

    /// A child might have children that need to be stored
    children: Vec<Rf<Node>>,
}

impl Node {
    pub fn new(ty: NodeType) -> Node {
        Node {
            ty,
            children: Vec::with_capacity(0),
        }
    }

    pub fn add_child(&mut self, node: impl Into<Rf<Node>>) {
        self.children.push(node.into())
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.ty.fmt(f)
    }
}

impl TreeDisplay for Node {
    fn num_children(&self) -> usize {
        self.children.len()
    }

    fn child_at(&self, index: usize) -> Option<Rf<dyn TreeDisplay>> {
        if index < self.children.len() {
            Some(Rf(self.children[index].clone()))
        } else {
            None
        }
    }
}

pub fn parse_from_stream<R>(stream: BufReader<R>)
where
    R: Read,
{
    let event_reader = EventReader::new(stream);

    let mut depth = 0;
    let mut nodes: HashMap<usize, Node> = HashMap::new();

    for e in event_reader {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if let Some(ty) = NodeType::try_node(name.local_name.as_str()) {
                    if depth == 0 {
                        nodes.insert(0, Node::new(ty));
                    } else {
                        nodes.insert(depth, Node::new(ty));
                    }
                }
                println!("{}+{}", indent(depth), name);
                depth += 1;
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                if depth == 0 {
                    continue;
                }

                let Some(to_add) = nodes.remove(&depth) else {
                    continue;
                };

                println!("{}-{}", indent(depth), name);

                if let Some(node) = nodes.get_mut(&(depth - 1)) {
                    node.add_child(to_add);
                }
            }
            Err(e) => {
                println!("Error: {:#}", e);
                break;
            }
            _ => {}
        }
    }

    println!("Nodes:\n{}", nodes.get(&0).unwrap().format());
}
