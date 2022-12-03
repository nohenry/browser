use std::{
    collections::HashMap,
    fmt::Display,
    io::{BufReader, Read},
};

use neb_errors::{DocumentError, DocumentErrorType};
use xml::{reader::XmlEvent, EventReader};

use crate::{tree_display::TreeDisplay, Rf};

fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

pub struct Document {
    errors: Vec<DocumentError>,

    head: Rf<Node>,

    body: Rf<Node>,
}

impl Document {
    pub fn get_errors(&self) -> &[DocumentError] {
        &self.errors
    }

    pub fn get_head(&self) -> Rf<Node> {
        self.head.clone()
    }
    
    pub fn get_body(&self) -> Rf<Node> {
        self.body.clone()
    }
}

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

    pub fn as_str(&self) -> &'static str {
        use NodeType::*;
        match self {
            Div => "div",
            Head => "head",
            Body => "body",
            Html => "html",
        }
    }
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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

    pub fn find_child_by_element_name(&self, name: &str) -> Option<Rf<Node>> {
        self.children
            .iter()
            .find(|f| f.borrow().as_ref().ty.as_str() == name)
            .map(|f| f.clone())
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
            Some(Rf(self.children[index].0.clone()))
        } else {
            None
        }
    }
}

pub fn parse_from_stream<R>(stream: BufReader<R>) -> Document
where
    R: Read,
{
    let event_reader = EventReader::new(stream);

    let mut depth = 0;
    let mut nodes: HashMap<usize, Node> = HashMap::new();
    let mut errors: Vec<DocumentError> = Vec::new();

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

    let (head, body) = if let Some(html) = nodes.get(&0) {
        let head = if let Some(head) = html.find_child_by_element_name("head") {
            head
        } else {
            errors.push(DocumentError::new(
                DocumentErrorType::ExpectedTag("head".into()),
                neb_errors::ErrorKind::Warning,
            ));
            Rf::new(Node::new(NodeType::Head))
        };

        let body = if let Some(body) = html.find_child_by_element_name("body") {
            body
        } else {
            errors.push(DocumentError::new(
                DocumentErrorType::ExpectedTag("body".into()),
                neb_errors::ErrorKind::Warning,
            ));
            Rf::new(Node::new(NodeType::Head))
        };

        (head, body)
    } else {
        errors.push(DocumentError::new(
            DocumentErrorType::ExpectedTag("html".into()),
            neb_errors::ErrorKind::Error,
        ));
        (
            Rf::new(Node::new(NodeType::Head)),
            Rf::new(Node::new(NodeType::Body)),
        )
    };

    Document { body, head, errors }

    // println!("Nodes:\n{}", .unwrap().format());
}
