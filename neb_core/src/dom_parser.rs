use std::{
    collections::HashMap,
    io::{BufReader, Read},
};

use neb_errors::{DocumentError, DocumentErrorType};
use neb_graphics::{drawing_context::DrawingContext, vello::kurbo::Rect};
use neb_util::{Rf, format::TreeDisplay};
use nom::{
    bytes::complete::tag,
    character::complete::digit1,
    combinator::{map, map_res, opt},
    multi::{count, separated_list1},
    sequence::preceded,
    IResult,
};
use xml::{reader::XmlEvent, EventReader};

use crate::{
    is_node,
    node::{Node, NodeType},
    styling::{parse_styles, Selector},
    svg::{self, PicoSvg},
};

pub fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

pub struct Document {
    errors: Vec<DocumentError>,

    head: Rf<Node>,

    body: Rf<Node>,

    styles: Rf<HashMap<String, Rf<Selector>>>,
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

    pub fn get_styles(&self) -> Rf<HashMap<String, Rf<Selector>>> {
        self.styles.clone()
    }
}

impl Document {
    pub fn draw(&self, dctx: &mut DrawingContext) {
        let body = self.body.borrow();
        body.draw(dctx, self);
    }
}

pub fn parse_from_stream<R>(stream: BufReader<R>) -> Document
where
    R: Read,
{
    let event_reader = EventReader::new(stream);

    let mut depth = 0;
    let mut nodes: HashMap<usize, Rf<Node>> = HashMap::new();
    let mut errors: Vec<DocumentError> = Vec::new();
    let mut styling = String::new();
    let mut svg: Option<(Rf<Node>, usize)> = None;

    for e in event_reader {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if let Some(ty) = NodeType::try_node(name.local_name.as_str()) {
                    if depth == 0 {
                        nodes.insert(0, Rf::new(Node::new_root(ty)));
                    } else {
                        match &ty {
                            NodeType::Path(s) => {
                                let d = attributes
                                    .iter()
                                    .find(|p| p.name.local_name == "d")
                                    .unwrap();
                                nodes.insert(
                                    depth,
                                    Rf::new(Node::new(
                                        NodeType::Path(d.value.clone()),
                                        nodes.get(&(depth - 1)).unwrap().clone(),
                                    )),
                                );
                            }
                            // NodeType::Div => {
                            //     let classes = attributes
                            //         .iter()
                            //         .find(|p| p.name.local_name == "class")
                            //         .unwrap();
                            //     let classes = classes
                            //         .value
                            //         .split_whitespace()
                            //         .map(|s| s.to_string())
                            //         .collect::<Vec<_>>();

                            //     let node = Node::new(ty, nodes.get(&(depth - 1)).unwrap().clone());
                            //     let node = node.with_classes(classes);

                            //     nodes.insert(depth, Rf::new(node));
                            // }
                            NodeType::Svg(_) => {
                                let view = attributes
                                    .iter()
                                    .find(|p| p.name.local_name == "viewBox")
                                    .unwrap();

                                let (_, rect) = parse_box(view.value.as_bytes()).unwrap();

                                let nd = Rf::new(Node::new(
                                    NodeType::Svg(PicoSvg {
                                        items: Vec::with_capacity(0),
                                        view: rect,
                                    }),
                                    nodes.get(&(depth - 1)).unwrap().clone(),
                                ));
                                svg = Some((nd.clone(), depth));
                                nodes.insert(depth, nd);
                            }
                            _ => {
                                nodes.insert(
                                    depth,
                                    Rf::new(Node::new(
                                        ty,
                                        nodes.get(&(depth - 1)).unwrap().clone(),
                                    )),
                                );
                            }
                        }
                    }
                }

                depth += 1;
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                if depth == 0 {
                    continue;
                }

                if let Some((node, sdepth)) = svg.clone() {
                    if sdepth == depth {
                        {
                            let mut node = node.borrow_mut();
                            let view = match &mut node.ty {
                                NodeType::Svg(s) => s.view,
                                _ => panic!(),
                            };
                            let svg = svg::PicoSvg::load1(&node, 0.5, view).unwrap();
                            node.ty = NodeType::Svg(svg);
                        }
                        {
                            let mut node = node.borrow_mut();
                            node.remove_children();
                        }
                        svg = None
                    } else {
                    }
                }

                let Some(to_add) = nodes.remove(&depth) else {
                    continue;
                };

                if let Some(node) = nodes.get_mut(&(depth - 1)) {
                    node.borrow_mut().add_child(to_add);
                }
            }
            Ok(XmlEvent::Characters(text)) => {
                let parent = nodes.get(&(depth - 1)).unwrap().clone();
                if let Some(node) = nodes.get_mut(&(depth - 1)) {
                    let ty = {
                        let ty = node.borrow();
                        ty.get_type().clone()
                    };
                    match ty {
                        NodeType::Style(_) => {
                            styling.push_str(text.trim());
                            styling.push('\n');
                            nodes.remove(&(depth - 1));
                        }
                        _ => {
                            let nd =
                                Rf::new(Node::new(NodeType::Text(text.trim().to_string()), parent));
                            node.borrow_mut().add_child_rf(nd);
                        }
                    }
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
        let head = if let Some(head) = html.borrow().find_child_by_element_name("head") {
            head
        } else {
            errors.push(DocumentError::new(
                DocumentErrorType::ExpectedTag("head".into()),
                neb_errors::ErrorKind::Warning,
            ));
            Rf::new(Node::new_root(NodeType::Head))
        };

        let body = if let Some(body) = html.borrow().find_child_by_element_name("body") {
            body
        } else {
            errors.push(DocumentError::new(
                DocumentErrorType::ExpectedTag("body".into()),
                neb_errors::ErrorKind::Warning,
            ));
            Rf::new(Node::new_root(NodeType::Head))
        };

        (head, body)
    } else {
        errors.push(DocumentError::new(
            DocumentErrorType::ExpectedTag("html".into()),
            neb_errors::ErrorKind::Error,
        ));
        (
            Rf::new(Node::new_root(NodeType::Head)),
            Rf::new(Node::new_root(NodeType::Body)),
        )
    };

    // let p: String = head
    //     .borrow()
    //     .iter()
    //     .filter(|f| is_node!(f.borrow(), NodeType::Style(_)))
    //     .map(|f| {
    //         let style = f.borrow_mut();
    //         match style.get_type() {
    //             NodeType::Style(txt) => txt.clone(),
    //             _ => panic!(),
    //         }
    //     })
    //     .intersperse("\n".to_string())
    //     .collect();

    println!("{}", styling);
    println!("{}", head.borrow().format());
    println!("{}", body.borrow().format());
    styling.push('\0');

    let styles = parse_styles(styling.as_str()).unwrap();
    dbg!(&styles);

    Document {
        body,
        head,
        errors,
        styles: Rf::new(styles),
    }
}

fn parse_box(bytes: &[u8]) -> IResult<&[u8], Rect> {
    map(
        separated_list1(tag(" "), nom::character::complete::i32),
        |vals| {
            Rect::new(
                vals[0] as f64,
                vals[1] as f64,
                vals[2] as f64,
                vals[3] as f64,
            )
        },
    )(bytes)
}
