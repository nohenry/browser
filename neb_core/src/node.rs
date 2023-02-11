use std::{collections::HashMap, fmt::Display, slice::Iter, sync::RwLockReadGuard};

use neb_graphics::{
    drawing_context::DrawingContext,
    simple_text,
    vello::{
        kurbo::{Affine, Rect, RoundedRectRadii},
        peniko::{Brush, Stroke},
    },
};
use neb_smf::{
    ast::Value,
    token::{SpannedToken, Token},
};

use crate::{
    rectr::RoundedRect,
    styling::{Align, ChildSizing, Direction},
    StyleValueAs,
};

use crate::{
    defaults,
    document::Document,
    ids::{get_id_mgr, ID},
    psize,
    styling::{StyleValue, UnitValue},
};
use neb_util::{
    format::{NodeDisplay, TreeDisplay},
    Rf,
};

/// The node type is a specific type of element
/// The most common element is the `Div` which is for general use case
#[derive(Clone)]
#[repr(u8)]
pub enum NodeType {
    Use(Vec<String>),
    StyleBlock,
    Setup,
    View {
        args: HashMap<String, Value>,
    },
    Style {
        name: String,
        properties: HashMap<String, Value>,
    },
    Text(String),
    Root,
}

impl NodeType {
    pub fn as_str(&self) -> &str {
        use NodeType::*;
        match self {
            Use(_) => "use",
            Setup => "setup",
            StyleBlock => "style",
            Text(s) => s.as_str(),
            View { .. } => "view",
            Root => "root",
            Style { name, .. } => name.as_str(),
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
    pub children: Vec<Rf<Node>>,

    /// An optional element for displaying
    pub element: Element,

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

    fn symbol_in_scope(&self, document: &Document, name: &str) -> Option<Rf<Node>> {
        let sty = self.children.iter().find_map(|f| {
            let node = f.borrow();
            match &node.ty {
                NodeType::Use(p) => {
                    if let Some(nd) = document.resolve_path(&document.get_body().borrow(), p.iter())
                    {
                        let b = {
                            let n = nd.borrow();
                            if n.ty.as_str() == name {
                                true
                            } else {
                                return n.symbol_in_scope(document, name);
                            }
                        };
                        if b {
                            return Some(nd);
                        }
                    }
                    None
                }
                _ => {
                    if node.ty.as_str() == name {
                        return Some(f.clone());
                    } else {
                        None
                    }
                }
            }
        });

        if sty.is_none() {
            if let Some(prent) = &self.parent {
                let p = prent.borrow();
                p.symbol_in_scope(document, name)
            } else {
                return None;
            }
        } else {
            return sty;
        }
    }

    pub fn styles(&self, document: &Document, key: &str) -> StyleValue {
        let class = match &self.ty {
            NodeType::View { args } => args.get("class"),
            _ => None,
        };

        match class {
            Some(Value::Ident(SpannedToken(_, Token::Ident(s)))) => {
                let parent = self.parent.as_ref().unwrap().borrow();
                let Some(symbol) = parent.symbol_in_scope(document, s) else {
                    return StyleValue::Empty
                };

                let sym = symbol.borrow();

                return StyleValue::from_symbol(&sym, key);
            }
            Some(Value::Array { values, .. }) => {
                for val in values.iter_items() {
                    if let Value::Ident(SpannedToken(_, Token::Ident(s))) = val {
                        let parent = self.parent.as_ref().unwrap().borrow();
                        let Some(symbol) = parent.symbol_in_scope(document, s) else {
                            return StyleValue::Empty
                        };

                        let sym = symbol.borrow();

                        match StyleValue::from_symbol(&sym, key) {
                            StyleValue::Empty => continue,
                            val => return val,
                        }
                    }
                }
            }
            _ => (),
        }

        StyleValue::Empty
    }

    pub fn bparent(&self) -> RwLockReadGuard<'_, Node> {
        self.parent.as_ref().unwrap().borrow()
    }

    pub fn is_displayed(&self) -> bool {
        match &self.ty {
            NodeType::View { .. } | NodeType::Text { .. } => true,
            _ => false,
        }
    }
}

impl NodeDisplay for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.ty, self.element.id)
    }
}

impl TreeDisplay<ID> for Node {
    fn num_children(&self) -> usize {
        self.children.len()
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay<ID>> {
        None
    }

    fn child_at_bx<'a>(&'a self, index: usize) -> Box<dyn TreeDisplay<ID> + 'a> {
        let p = self.children.iter().nth(index).unwrap().borrow();

        Box::new(p)
    }

    fn get_user_data(&self) -> Option<ID> {
        Some(self.element.id)
    }
}

/// An element is the part that is displayed on the screen
///
/// This struct containts layout; size information
///
/// Nodes can contain element (or not)
#[derive(Clone)]
pub struct Element {
    id: ID,

    classes: Vec<String>,
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("id", &self.id)
            .field("classes", &self.classes)
            .finish()
    }
}

impl Element {
    pub fn new() -> Self {
        Element {
            id: get_id_mgr().gen_insert_zero(),
            classes: Vec::with_capacity(0),
        }
    }

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

        let child_sizing = StyleValueAs!(node.styles(document, "childSizing"), ChildSizing)
            .unwrap_or(ChildSizing::Individual);

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
        let layout_children_vertically = |bounds: &Rect, gap: UnitValue, fit: bool| {
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

            let mut max_width = 0;
            // Layout each child and add it's requested size to the total area
            for child in node.children.iter() {
                let node = child.borrow();
                if !node.is_displayed() {
                    continue;
                }
                // dbg!(node.element.id);

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0, bounds.y0 + rect.height(), bounds.x1, bounds.y1);

                let area = node.element.layout(&node, area, depth + 1, document);
                if area.x1 as i32 > max_width {
                    max_width = area.x1 as i32;
                }
                if fit {
                    if area.width() > rect.width() {
                        rect.x1 = rect.x0 + area.width();
                    } else if area.x1 > rect.x1 {
                        rect.x1 = area.x1
                    }
                }

                // We round height for that pixel perfection 🤤
                rect.y1 += area.height().round() + gap_pixels as f64
            }
            if let ChildSizing::Match = child_sizing {
                // set layout for all children with max width
                for child in node.children.iter() {
                    let node = child.borrow();
                    if !node.is_displayed() {
                        continue;
                    }

                    node.element.layout(&node, rect, depth + 1, document);

                    // let mut manager = get_id_mgr();
                    // let mut layout = *manager.get_layout(node.element.id);
                    // if max_width > layout.content_rect.width() as i32 {
                    //     layout.content_rect.x1 +=
                    //         (max_width - layout.content_rect.width() as i32) as f64;
                    //     layout.border_rect.x1 +=
                    //         (max_width - layout.border_rect.width() as i32) as f64;
                    // }
                    // manager.set_layout_content(node.element.id, layout.content_rect);
                    // manager.set_layout_border(node.element.id, layout.border_rect);
                }
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
                if !node.is_displayed() {
                    continue;
                }

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1 - rect.height());

                let area = self.layout(&node, area, depth + 1, document);
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
                if !node.is_displayed() {
                    continue;
                }

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0 + rect.width(), bounds.y0, bounds.x1, bounds.y1);

                let area = self.layout(&node, area, depth + 1, document);
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
                if !node.is_displayed() {
                    continue;
                }

                // The bounds of the space that has not been taken up yet
                let area = Rect::new(bounds.x0, bounds.y0, bounds.x1 - rect.width(), bounds.y1);

                let area = self.layout(&node, area, depth + 1, document);
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
            NodeType::View { .. } => {
                let gap = StyleValueAs!(node.styles(document, "gap"), Gap)
                    .unwrap_or(UnitValue::Pixels(defaults::GAP));

                let direction = StyleValueAs!(node.styles(document, "direction"), Direction)
                    .unwrap_or(defaults::DIRECTION);

                let fit = true;

                let align = StyleValueAs!(node.styles(document, "align"), Align);

                let area = match (direction, align) {
                    (Direction::Vertical, _) => layout_children_vertically(&bounds, gap, fit),
                    (Direction::VerticalReverse, _) => layout_children_vertically_rev(gap, fit),
                    (Direction::Horizontal, _) => layout_children_horizontally(gap, fit),
                    (Direction::HorizontalReverse, _) => layout_children_horizontally_rev(gap, fit),
                };

                let area = match StyleValueAs!(node.styles(document, "align"), Align) {
                    Some(Align::Right) => {
                        Rect::new(bounds.x1 - area.width(), area.y0, bounds.x1, area.y1)
                    }
                    Some(Align::Center) => Rect::new(
                        bounds.width() / 2.0 - area.width() / 2.0 + bounds.x0,
                        area.y0,
                        bounds.width() / 2.0 + area.width() / 2.0 + bounds.x0,
                        area.y1,
                    ),
                    _ => area,
                };

                let area = match (direction, align) {
                    (Direction::Vertical, _) => layout_children_vertically(&area, gap, fit),
                    (Direction::VerticalReverse, _) => layout_children_vertically_rev(gap, fit),
                    (Direction::Horizontal, _) => layout_children_horizontally(gap, fit),
                    (Direction::HorizontalReverse, _) => layout_children_horizontally_rev(gap, fit),
                };

                area
            }
            // SymbolKind::Node { args }
            // NodeType::Svg(svg) => {
            //     println!("{:?} {}", svg.view, svg.view.width());
            //     Rect::from_origin_size(
            //         (bounds.x0, bounds.y0),
            //         (svg.view.width(), svg.view.height()),
            //     )
            // }
            NodeType::Text(t) => {
                let mut simple_text = simple_text::SimpleText::new();
                let tl = simple_text.layout(None, psize!(defaults::TEXT_SIZE), t, &bounds);

                let area =
                    Rect::from_origin_size((bounds.x0, bounds.y0), (tl.width(), tl.height()));

                // let area = match StyleValueAs!(
                //     node.parent
                //         .as_ref()
                //         .unwrap()
                //         .borrow()
                //         .styles(document, "align"),
                //     Align
                // ) {
                //     Some(Align::Right) => {
                //         Rect::new(bounds.x1 - area.width(), area.y0, bounds.x1, area.y1)
                //     }
                //     Some(Align::Center) => Rect::new(
                //         bounds.width() / 2.0 - area.width() / 2.0 + bounds.x0,
                //         area.y0,
                //         bounds.width() / 2.0 + area.width() / 2.0 + bounds.x0,
                //         area.y1,
                //     ),
                //     _ => area,
                // };
                // .map(|r| r.try_into().unwrap());

                // let x_offset = match align {
                //     Some(TextAlign::Center) => tl.width() / 2.0,
                //     Some(TextAlign::Left) => 0.0,
                //     _ => 0.0,
                // };
                area
            }
            NodeType::Root => {
                let gap = StyleValueAs!(node.styles(document, "gap"), Gap)
                    .unwrap_or(UnitValue::Pixels(defaults::GAP));

                let direction = StyleValueAs!(node.styles(document, "direction"), Direction)
                    .unwrap_or(defaults::DIRECTION);

                let fit = false;
                match direction {
                    Direction::Vertical => layout_children_vertically(&bounds, gap, fit),
                    Direction::VerticalReverse => layout_children_vertically_rev(gap, fit),
                    Direction::Horizontal => layout_children_horizontally(gap, fit),
                    Direction::HorizontalReverse => layout_children_horizontally_rev(gap, fit),
                };

                /* Only difference in body is in keeps the max size */
                bounds
            }
            _ => Rect::ZERO,
        };

        get_id_mgr().set_layout_padding(node.element.id, area);

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
        get_id_mgr().set_layout_content(node.element.id, bounds);

        let bounds = if let Some(border) = border_width {
            Rect::new(
                bounds.x0 - border.x0,
                bounds.y0 - border.y0,
                bounds.x1 + border.x1,
                bounds.y1 + border.y1,
            )
        } else {
            bounds
        };

        // Set the border bounds; the physical area that the border takes up. This bounds is used or drawing the border color
        get_id_mgr().set_layout_border(node.element.id, bounds);

        bounds
    }

    pub fn draw(&self, node: &Node, dctx: &mut DrawingContext, document: &Document) {
        if !node.is_displayed() {
            return;
        }
        let binding = get_id_mgr();
        let layout = binding.get_layout(self.id);

        let background_color =
            StyleValueAs!(node.styles(document, "backgroundColor"), BackgroundColor);
        let border_color = StyleValueAs!(node.styles(document, "borderColor"), BorderColor);
        let border_width =
            StyleValueAs!(node.styles(document, "borderWidth"), BorderWidth).unwrap_or_default();

        let foreground_color =
            StyleValueAs!(node.styles(document, "foregroundColor"), ForegroundColor);

        let parent_fg_col = node.parent.as_ref().and_then(|parent| {
            StyleValueAs!(
                parent.borrow().styles(document, "foregroundColor"),
                ForegroundColor
            )
        });

        let radius = StyleValueAs!(node.styles(document, "radius"), Radius);

        let radius: Option<RoundedRectRadii> = radius.map(|rad| rad.try_into().unwrap());

        if let Some(color) = border_color {
            // If we have a radius, draw it instead
            if let Some(radius) = radius {
                let _rounded = RoundedRect::from_rect(layout.border_rect, radius);
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

        let parent_foreground_color = if let Some(foreground_color) = parent_fg_col {
            foreground_color
        } else {
            foreground_color
        };

        // let node = node.borrow();

        match &node.ty {
            // _ => ()
            // NodeType::Svg(svg) => {
            //     for item in &svg.items {
            //         match item {
            //             svg::Item::Fill(fill) => {
            //                 dctx.builder.fill(
            //                     Fill::NonZero,
            //                     Affine::IDENTITY,
            //                     fill.color,
            //                     None,
            //                     &fill.path,
            //                 );
            //             }
            //             svg::Item::Stroke(stroke) => {
            //                 dctx.builder.stroke(
            //                     &Stroke::new(stroke.width as f32),
            //                     Affine::IDENTITY,
            //                     stroke.color,
            //                     None,
            //                     &stroke.path,
            //                 );
            //             }
            //             svg::Item::Path(path) => {
            //                 dctx.builder.fill(
            //                     neb_graphics::vello::peniko::Fill::NonZero,
            //                     Affine::translate(Vec2::new(-svg.view.x0, -svg.view.y0))
            //                         * Affine::translate(Vec2::new(
            //                             layout.content_rect.x0,
            //                             layout.content_rect.y0,
            //                         )),
            //                     &Brush::Solid(foreground_color),
            //                     None,
            //                     &path,
            //                 );
            //             }
            //         }
            //     }
            // }
            NodeType::Text(t) => {
                dctx.text.add(
                    &mut dctx.builder,
                    None,
                    psize!(defaults::TEXT_SIZE),
                    Some(&Brush::Solid(parent_foreground_color)),
                    Affine::translate((layout.content_rect.x0, layout.content_rect.y0)),
                    t,
                    &layout.content_rect,
                );
            }
            _ => (),
        }
    }
}
