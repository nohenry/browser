use std::{
    cell::Ref,
    collections::HashMap,
    fmt::Display,
    slice::Iter,
    sync::{MutexGuard, RwLockReadGuard},
};

use neb_graphics::{
    drawing_context::DrawingContext,
    simple_text,
    vello::{
        kurbo::{Affine, Rect, RoundedRectRadii, Size, Vec2},
        peniko::{Brush, Color, Fill, Stroke},
    },
};

use crate::{
    rectr::RoundedRect,
    styling::{self, Direction},
    svg::{self, PicoSvg},
    StyleValueAs,
};

use crate::{
    defaults,
    dom_parser::Document,
    ids::{get_id_mgr, ID},
    psize,
    styling::{Selector, StyleValue, UnitValue},
};
use neb_util::{Rf, format::{TreeDisplay, NodeDisplay}};

/// The node type is a specific type of element
/// The most common element is the `Div` which is for general use case
#[derive(Clone)]
#[repr(u8)]
pub enum NodeType {
    /// A general element type
    Div = 0,

    Span,

    Head,

    Body,

    Html,

    Style(String),

    Text(String),

    Svg(PicoSvg),

    G,

    Path(String),

    Root,
}

impl NodeType {
    pub fn try_node(element: &str) -> Option<NodeType> {
        use NodeType::*;

        match element.to_lowercase().as_str() {
            "div" => Some(Div),
            "span" => Some(Span),
            "html" => Some(Html),
            "head" => Some(Head),
            "body" => Some(Body),
            "style" => Some(Style(String::with_capacity(0))),
            "g" => Some(G),
            "path" => Some(Path(String::with_capacity(0))),
            "svg" => Some(Svg(PicoSvg {
                items: Vec::with_capacity(0),
                view: Rect::ZERO,
            })),
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
            Span => "span",
            Head => "head",
            Body => "body",
            Html => "html",
            Style(_) => "style",
            Svg(_) => "svg",
            G => "g",
            Path(_) => "path",
            Text(s) => s.as_str(),
            Root => "root",
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
#[derive(Clone)]
pub struct Node {
    /// The specific type that this node represents
    pub ty: NodeType,

    /// A child might have children that need to be stored
    children: Vec<Rf<Node>>,

    /// An optional element for displaying
    element: Element,

    parent: Option<Rf<Node>>,
}

impl Node {
    pub fn new(ty: NodeType, parent: Rf<Node>) -> Node {
        Node {
            ty,
            children: Vec::with_capacity(0),
            element: Element::default(),
            parent: Some(parent),
        }
    }

    pub fn new_root(ty: NodeType) -> Node {
        Node {
            ty,
            children: Vec::with_capacity(0),
            element: Element::default(),
            parent: None,
        }
    }

    pub fn with_type(mut self, ty: NodeType) -> Self {
        self.ty = ty;
        self
    }

    pub fn with_classes(mut self, classes: impl Into<Vec<String>>) -> Self {
        self.element = self.element.with_classes(classes);
        self
    }

    pub fn remove_children(&mut self) {
        let remove = match self.ty {
            NodeType::Svg(_) => true,
            _ => false,
        };

        if remove {
            self.children.clear();
        }
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
        std::mem::discriminant(&self.ty) == std::mem::discriminant(ty)
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

    pub fn parent(&self) -> Rf<Node> {
        self.parent.as_ref().expect("Expected parent!").clone()
    }

    pub fn styles(&self, document: &Document, key: &str) -> StyleValue {
        let styles = document.get_styles();
        let styles = styles.borrow();
        let styles = styles.get(self.get_type().as_str());
        if let Some(styles) = styles {
            let styles = styles.borrow();
            styles.get(key).cloned().unwrap_or_else(|| {
                if self.parent.is_some() && styling::is_inherited(key) {
                    self.bparent().styles(document, key)
                } else {
                    StyleValue::Empty
                }
            })
        } else {
            if self.parent.is_some() && styling::is_inherited(key) {
                self.bparent().styles(document, key)
            } else {
                StyleValue::Empty
            }
        }
    }

    pub fn bparent(&self) -> RwLockReadGuard<'_, Node> {
        self.parent.as_ref().unwrap().borrow()
    }
}

impl NodeDisplay for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // self.ty.fmt(f)
        write!(f, "{} - {}", self.ty, self.element.id)
    }
}

impl TreeDisplay for Node {
    fn num_children(&self) -> usize {
        self.children.len()
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        if index < self.children.len() {
            Some(&self.children[index])
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

    classes: Vec<String>,
}

impl Element {
    pub fn with_classes(mut self, classes: impl Into<Vec<String>>) -> Self {
        self.classes = classes.into();
        self
    }
}

impl Default for Element {
    fn default() -> Self {
        Self {
            id: get_id_mgr().gen_insert_zero(),
            classes: Vec::with_capacity(0),
        }
    }
}

impl Element {
    pub fn layout(&self, node: &Node, bounds: Rect, depth: usize, document: &Document) -> Rect {
        let padding: Option<Rect> =
            StyleValueAs!(node.styles(document, "padding"), Padding).map(|r| r.try_into().unwrap());
        let border_width: Option<Rect> =
            StyleValueAs!(node.styles(document, "borderWidth"), BorderWidth)
                .map(|r| r.try_into().unwrap());

        /*
            The padding and border take up space,
            therefore we have to subtract them from the bounds so that
            the child nodes don't use up this space
        */
        let bounds = if let Some(padding) = padding {
            Rect::new(
                bounds.x0 + padding.x0,
                bounds.y0 + padding.y0,
                bounds.x1 - padding.x1,
                bounds.y1 - padding.y1,
            )
        } else {
            bounds
        };

        let bounds = if let Some(border) = border_width {
            Rect::new(
                bounds.x0 + border.x0,
                bounds.y0 + border.y0,
                bounds.x1 - border.x1,
                bounds.y1 - border.y1,
            )
        } else {
            bounds
        };

        // Lays out child nodes in a stack
        let layout_children_vertically = |gap: UnitValue, fit: bool| {
            // Start the bounds from top up (bounds.y0)
            let mut rect = Rect::new(
                bounds.x0,
                bounds.y0,
                if fit { bounds.x0 } else { bounds.x1 },
                bounds.y0,
            );

            let gap_pixels = match gap {
                UnitValue::Pixels(p) => p,
            };

            // Layout each child and add it's requested size to the total area
            for child in node.children.iter() {
                let node = child.borrow();

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0, bounds.y0 + rect.height(), bounds.x1, bounds.y1);

                let area = node.element.layout(&node, area, depth + 1, document);
                if fit {
                    if area.width() > rect.width() {
                        rect.x1 = rect.x0 + area.width();
                    }
                }

                // We round height for that pixel perfection 🤤
                rect.y1 += area.height().round() + gap_pixels as f64
            }
            rect
        };

        // Lays out child nodes in a stack
        let layout_children_vertically_rev = |gap: UnitValue, fit: bool| {
            // Start the bounds from top up (bounds.y0)
            let mut rect = Rect::new(
                bounds.x0,
                bounds.y1,
                if fit { bounds.x0 } else { bounds.x1 },
                bounds.y1,
            );

            let gap_pixels = match gap {
                UnitValue::Pixels(p) => p,
            };

            // Layout each child and add it's requested size to the total area
            for child in node.children.iter() {
                let node = child.borrow();

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1 - rect.height());

                let area = node.element.layout(&node, area, depth + 1, document);
                if fit {
                    if area.width() > rect.width() {
                        rect.x1 = rect.x0 + area.width();
                    }
                }

                // We round height for that pixel perfection 🤤
                rect.y0 -= area.height().round() + gap_pixels as f64
            }
            rect
        };

        // Lays out child nodes in a stack
        let layout_children_horizontally = |gap: UnitValue, fit: bool| {
            // Start the bounds from top up (bounds.y0)
            let mut rect = Rect::new(
                bounds.x0,
                bounds.y0,
                bounds.x0,
                if fit { bounds.y0 } else { bounds.y1 },
            );

            // The gap is the space in between child nodes
            let gap_pixels = match gap {
                UnitValue::Pixels(p) => p,
            };

            // Layout each child and add it's requested size to the total area
            for child in node.children.iter() {
                let node = child.borrow();

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0 + rect.width(), bounds.y0, bounds.x1, bounds.y1);

                let area = node.element.layout(&node, area, depth + 1, document);
                if fit {
                    if area.height() > rect.height() {
                        rect.y1 = rect.y0 + area.height();
                    }
                }

                // We round height for that pixel perfection 🤤
                rect.x1 += area.width().round() + gap_pixels as f64
            }
            rect
        };

        // Lays out child nodes in a stack
        let layout_children_horizontally_rev = |gap: UnitValue, fit: bool| {
            // Start the bounds from top up (bounds.y0)
            let mut rect = Rect::new(
                bounds.x1,
                bounds.y0,
                bounds.x1,
                if fit { bounds.y0 } else { bounds.y1 },
            );

            // The gap is the space in between child nodes
            let gap_pixels = match gap {
                UnitValue::Pixels(p) => p,
            };

            // Layout each child and add it's requested size to the total area
            for child in node.children.iter() {
                let node = child.borrow();

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0, bounds.y0, bounds.x1 - rect.width(), bounds.y1);

                let area = node.element.layout(&node, area, depth + 1, document);
                if fit {
                    if area.height() > rect.height() {
                        rect.y1 = rect.y0 + area.height();
                    }
                }

                // We round height for that pixel perfection 🤤
                rect.x0 -= area.width().round() + gap_pixels as f64
            }
            rect
        };

        let area = match &node.ty {
            NodeType::Svg(svg) => {
                println!("{:?} {}", svg.view, svg.view.width());
                Rect::from_origin_size(
                    (bounds.x0, bounds.y0),
                    (svg.view.width(), svg.view.height()),
                )
            }
            NodeType::Text(t) => {
                let mut simple_text = simple_text::SimpleText::new();
                let tl = simple_text.layout(None, psize!(defaults::TEXT_SIZE), t);
                Rect::from_origin_size((bounds.x0, bounds.y0), (tl.width(), tl.height()))
            }
            NodeType::Div | NodeType::Span => {
                let gap = StyleValueAs!(node.styles(document, "gap"), Gap)
                    .unwrap_or(UnitValue::Pixels(defaults::GAP));

                let direction = StyleValueAs!(node.styles(document, "direction"), Direction)
                    .unwrap_or(defaults::DIRECTION);

                let fit = true;

                match direction {
                    Direction::Vertical => layout_children_vertically(gap, fit),
                    Direction::VerticalReverse => layout_children_vertically_rev(gap, fit),
                    Direction::Horizontal => layout_children_horizontally(gap, fit),
                    Direction::HorizontalReverse => layout_children_horizontally_rev(gap, fit),
                }
            }
            NodeType::Body => {
                let gap = StyleValueAs!(node.styles(document, "gap"), Gap)
                    .unwrap_or(UnitValue::Pixels(defaults::GAP));

                let direction = StyleValueAs!(node.styles(document, "direction"), Direction)
                    .unwrap_or(defaults::DIRECTION);

                let fit = false;
                match direction {
                    Direction::Vertical => layout_children_vertically(gap, fit),
                    Direction::VerticalReverse => layout_children_vertically_rev(gap, fit),
                    Direction::Horizontal => layout_children_horizontally(gap, fit),
                    Direction::HorizontalReverse => layout_children_horizontally_rev(gap, fit),
                };

                /* Only difference in body is in keeps the max size */
                bounds
            }
            _ => Rect::ZERO,
        };

        let bounds = if let Some(padding) = padding {
            Rect::new(
                area.x0 - padding.x0,
                area.y0 - padding.y0,
                area.x1 + padding.x1,
                area.y1 + padding.y1,
            )
        } else {
            area
        };

        // Set the content bounds. This is used for drawing a background for the content with a border
        get_id_mgr().set_layout_content(self.id, bounds);

        let bounds = if let Some(border) = border_width {
            Rect::new(
                area.x0 - border.x0,
                area.y0 - border.y0,
                area.x1 + border.x1,
                area.y1 + border.y1,
            )
        } else {
            bounds
        };

        // Set the border bounds; the physical area that the border takes up. This bounds is used or drawing the border color
        get_id_mgr().set_layout_border(self.id, bounds);

        bounds
    }

    pub fn draw(&self, node: &Node, dctx: &mut DrawingContext, document: &Document) {
        let mut binding = get_id_mgr();
        let layout = binding.get_layout(self.id);

        let background_color =
            StyleValueAs!(node.styles(document, "backgroundColor"), BackgroundColor);
        let border_color = StyleValueAs!(node.styles(document, "borderColor"), BorderColor);
        let border_width =
            StyleValueAs!(node.styles(document, "borderWidth"), BorderWidth).unwrap_or_default();

        let foreground_color =
            StyleValueAs!(node.styles(document, "foregroundColor"), ForegroundColor);

        let radius = StyleValueAs!(node.styles(document, "radius"), Radius);

        let radius: Option<RoundedRectRadii> = radius.map(|rad| rad.try_into().unwrap());

        if let Some(color) = border_color {
            // If we have a radius, draw it instead
            if let Some(radius) = radius {
                let rounded = RoundedRect::from_rect(layout.border_rect, radius);
                // dctx.builder.fill(
                //     neb_graphics::vello::peniko::Fill::NonZero,
                //     Affine::IDENTITY,
                //     color,
                //     None,
                //     &rounded,
                // );
            } else {
                // let width = match border_width {
                //     UnitValue::Pixels(p) => p,
                // };
                let r: Rect = border_width.try_into().unwrap();
                // No radius
                dctx.builder.stroke(
                    &Stroke::new(r.x0 as _),
                    Affine::IDENTITY,
                    color,
                    None,
                    &layout.border_rect,
                );
                // dctx.builder.fill(
                //     neb_graphics::vello::peniko::Fill::NonZero,
                //     Affine::IDENTITY,
                //     color,
                //     None,
                //     &layout.border_rect,
                // );
            }
        }

        if let Some(color) = background_color {
            if let Some(radius) = radius {
                let border_width = StyleValueAs!(node.styles(document, "borderWidth"), BorderWidth);

                // Only allow the content to have a radius if the radius is larger than the border width
                let radius = if let Some(w) = border_width {
                    let w: Rect = w.try_into().unwrap();
                    RoundedRectRadii::new(
                        if radius.top_left > w.x0 && radius.top_left > w.y0 {
                            radius.top_left
                        } else {
                            0.0
                        },
                        if radius.top_right > w.x1 && radius.top_right > w.y0 {
                            radius.top_left
                        } else {
                            0.0
                        },
                        if radius.bottom_right > w.x1 && radius.bottom_right > w.y1 {
                            radius.top_left
                        } else {
                            0.0
                        },
                        if radius.bottom_left > w.x0 && radius.bottom_left > w.y0 {
                            radius.top_left
                        } else {
                            0.0
                        },
                    )
                } else {
                    radius
                };

                let mut rounded = RoundedRect::from_rect(layout.content_rect, radius);
                rounded.set_center(layout.border_rect);

                dctx.builder.fill(
                    neb_graphics::vello::peniko::Fill::NonZero,
                    Affine::IDENTITY,
                    color,
                    None,
                    &rounded,
                );
            } else {
                // No radius
                dctx.builder.fill(
                    neb_graphics::vello::peniko::Fill::NonZero,
                    Affine::IDENTITY,
                    color,
                    None,
                    &layout.content_rect,
                );
            }
        }

        let foreground_color = if let Some(foreground_color) = foreground_color {
            foreground_color
        } else {
            defaults::FOREGROUND_COLOR
        };

        match &node.ty {
            NodeType::Svg(svg) => {
                for item in &svg.items {
                    match item {
                        svg::Item::Fill(fill) => {
                            dctx.builder.fill(
                                Fill::NonZero,
                                Affine::IDENTITY,
                                fill.color,
                                None,
                                &fill.path,
                            );
                        }
                        svg::Item::Stroke(stroke) => {
                            dctx.builder.stroke(
                                &Stroke::new(stroke.width as f32),
                                Affine::IDENTITY,
                                stroke.color,
                                None,
                                &stroke.path,
                            );
                        }
                        svg::Item::Path(path) => {
                            dctx.builder.fill(
                                neb_graphics::vello::peniko::Fill::NonZero,
                                Affine::translate(Vec2::new(-svg.view.x0, -svg.view.y0))
                                    * Affine::translate(Vec2::new(
                                        layout.content_rect.x0,
                                        layout.content_rect.y0,
                                    )),
                                &Brush::Solid(foreground_color),
                                None,
                                &path,
                            );
                        }
                    }
                }
            }
            NodeType::Text(t) => {
                dctx.text.add(
                    &mut dctx.builder,
                    None,
                    psize!(defaults::TEXT_SIZE),
                    None,
                    None,
                    Some(&Brush::Solid(foreground_color)),
                    Affine::translate((layout.content_rect.x0, layout.content_rect.y0)),
                    t,
                );
            }
            _ => (),
        }
    }
}
