use std::{fmt::Display, slice::Iter};

use neb_graphics::{
    drawing_context::DrawingContext,
    piet_scene::{
        kurbo::{Affine, Rect, Size},
        Brush, Color
    },
    simple_text,
};

use crate::{
    defaults,
    ids::{get_id_mgr, ID},
    psize,
    tree_display::TreeDisplay,
    Rf, dom_parser::Document,
};

/// The node type is a specific type of element
/// The most common element is the `Div` which is for general use case
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum NodeType {
    /// A general element type
    Div = 0,

    Head,

    Body,

    Html,

    Style(String),

    Text(String),
}

impl NodeType {
    pub fn try_node(element: &str) -> Option<NodeType> {
        use NodeType::*;

        match element.to_lowercase().as_str() {
            "div" => Some(Div),
            "html" => Some(Html),
            "head" => Some(Head),
            "body" => Some(Body),
            "style" => Some(Style(String::with_capacity(0))),
            _ => None,
        }
    }

    pub fn try_node_poss(element: Option<&str>) -> Option<NodeType> {
        match element {
            Some(node) => NodeType::try_node(node),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        use NodeType::*;
        match self {
            Div => "div",
            Head => "head",
            Body => "body",
            Html => "html",
            Style(_) => "style",
            Text(s) => s.as_str(),
        }
    }
}

#[macro_export]
macro_rules! is_node {
    ($expression:expr, $(|)? $( $pattern:pat_param)|+ $( if $guard: expr )? $(,)?) => {{
        match $expression.get_type() {
            $( $pattern )|+ $( if $guard )? => true,
            _ => false
        }
    }};
}


impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A node that represents an element in the document tree
#[derive(Debug, Clone)]
pub struct Node {
    /// The specific type that this node represents
    ty: NodeType,

    /// A child might have children that need to be stored
    children: Vec<Rf<Node>>,

    /// An optional element for displaying
    element: Element,
}

impl Node {
    pub fn new(ty: NodeType) -> Node {
        Node {
            ty,
            children: Vec::with_capacity(0),
            element: Element::default(),
        }
    }

    pub fn with_type(mut self, ty: NodeType) -> Self {
        self.ty = ty;
        self
    }

    pub fn add_child(&mut self, node: impl Into<Rf<Node>>) {
        self.children.push(node.into())
    }

    pub fn add_child_rf(&mut self, node: Rf<Node>) {
        self.children.push(node)
    }

    pub fn find_child_by_element_name(&self, name: &str) -> Option<Rf<Node>> {
        self.children
            .iter()
            .find(|f| f.borrow().ty.as_str() == name)
            .map(|f| f.clone())
    }

    pub fn iter(&self) -> Iter<Rf<Node>> {
        self.children.iter()
    }

    pub fn get_element(&self) -> &Element {
        &self.element
    }

    pub fn is_type(&self, ty: &NodeType) -> bool {
        std::mem::discriminant(&self.ty) ==  std::mem::discriminant(ty)
    }

    pub fn get_type(&self) -> &NodeType {
        &self.ty
    }

    pub fn get_element_mut(&mut self) -> &mut Element {
        &mut self.element
    }

    pub fn draw(&self, dctx: &mut DrawingContext, document: &Document) {
        self.element.draw(self, dctx, document);

        self.children
            .iter()
            .for_each(|child| child.borrow().draw(dctx, document));

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

/// An element is the part that is displayed on the screen
///
/// This struct containts layout; size information
///
/// Nodes can contain element (or not)
#[derive(Debug, Clone)]
pub struct Element {
    id: ID,

    pub content_size: Size,
    pub padding: Rect,
    pub border: Rect,

    pub background_color: Option<Brush>,
    pub foreground_color: Brush,
}

impl Default for Element {
    fn default() -> Self {
        Self {
            id: get_id_mgr().gen_insert_zero(),

            content_size: Default::default(),
            padding: Default::default(),
            border: Default::default(),

            background_color: Some(Color::BEIGE.into()),
            foreground_color: Color::DARK_RED.into(),
        }
    }
}

impl Element {
    pub fn layout(&self, node: &Node, bounds: Rect, depth: usize) -> Rect {
        // println!("Layout: {}", bounds);
        // println!("\n{}layout: {}", indent(depth), node.ty.as_str());
        let bounds = Rect::new(
            bounds.x0 + self.padding.x0,
            bounds.y0 + self.padding.y0,
            bounds.x1 + self.padding.x1,
            bounds.y1 + self.padding.y1,
        );

        let area = match &node.ty {
            NodeType::Text(t) => {
                let mut simple_text = simple_text::SimpleText::new();
                let tl = simple_text.layout(None, psize!(defaults::TEXT_SIZE), t);
                Rect::from_origin_size((bounds.x0, bounds.y0), (tl.width(), tl.height()))
            }
            NodeType::Div => {
                let mut rect = Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y0);

                // println!("{}Start", indent(depth));
                for child in node.children.iter() {
                    // println!("{}lp", indent(depth));
                    let node = child.borrow();

                    let area =
                        Rect::new(bounds.x0, bounds.y0 + rect.height(), bounds.x1, bounds.y1);
                    // println!(
                    //     "{}dfjsklsdj: {:?}\n{}{:?}",
                    //     indent(depth),
                    //     area,
                    //     indent(depth),
                    //     rect
                    // );

                    let area = node.element.layout(&node, area, depth + 1);
                    // println!("{}Nsdfsdode: {:?}", indent(depth), area);

                    rect.y1 += area.height()
                    // rect = area;
                }
                // println!("{}End", indent(depth));
                rect
            }
            NodeType::Body => {
                let mut rect = Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y0);

                for child in node.children.iter() {
                    let node = child.borrow();
                    let area =
                        Rect::new(bounds.x0, bounds.y0 + rect.height(), bounds.x1, bounds.y1);

                    let area = node.element.layout(&node, area, depth + 1);

                    rect.y1 += area.height()
                }
                bounds
            }
            _ => Rect::ZERO,
        };

        let padded = Rect::new(
            area.x0 - self.padding.x0,
            area.y0 - self.padding.y0,
            area.x1 - self.padding.x1,
            area.y1 - self.padding.y1,
        );

        get_id_mgr().set_layout(self.id, padded);

        // println!();
        // println!("Node: {:20} Layout: {}", node.ty.as_str(), padded);
        padded
    }

    pub fn draw(&self, node: &Node, dctx: &mut DrawingContext, document: &Document) {
        let mut binding = get_id_mgr();
        let layout = binding.get_layout(self.id);

        if let Some(bg) = &self.background_color {
            dctx.builder.fill(
                neb_graphics::piet_scene::Fill::NonZero,
                Affine::IDENTITY,
                bg,
                None,
                &layout,
            );
        }

        match &node.ty {
            NodeType::Text(t) => {
                dctx.text.add(
                    &mut dctx.builder,
                    None,
                    psize!(defaults::TEXT_SIZE),
                    None,
                    None,
                    Some(&self.foreground_color),
                    Affine::translate((layout.x0, layout.y0)),
                    t,
                );
            }
            _ => (),
        }
    }
}
