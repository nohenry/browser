use std::collections::HashSet;
use std::fmt::{Debug, Display};

use neb_graphics::vello::kurbo::{Rect, RoundedRectRadii};
use neb_graphics::vello::peniko::Color;
use neb_macros::EnumHash;
use neb_smf::ast::{ElementArgs, Value};
use neb_smf::token::{SpannedToken, Token, Unit};

use crate::node::{Node, NodeType};

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Vertical,
    Horizontal,
    VerticalReverse,
    HorizontalReverse,
}

lazy_static::lazy_static! {
    static ref INHERITED: HashSet<&'static str> = HashSet::from(["foregroundColor"]);
}

pub fn is_inherited(key: &str) -> bool {
    INHERITED.contains(key)
}

#[derive(EnumHash, Debug, Clone)]
pub enum StyleValue {
    /* Colors */
    BackgroundColor { color: Color },
    ForegroundColor { color: Color },

    BorderWidth { rect: UnitRect },
    BorderColor { color: Color },

    /* Sizing */
    Gap { amount: UnitValue },
    Padding { rect: UnitRect },
    Radius { rect: UnitRect },
    Direction { direction: Direction },

    Empty,
}

pub fn color_from_iter<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Option<Color> {
    let r = iter.next()?;
    let g = iter.next()?;
    let b = iter.next()?;
    let a = iter.next();
    match (r, g, b, a) {
        (
            Value::Integer(r, None, _),
            Value::Integer(g, None, _),
            Value::Integer(b, None, _),
            None,
        ) => Some(Color {
            r: *r as _,
            g: *g as _,
            b: *b as _,
            a: 255,
        }),
        (
            Value::Integer(r, None, _),
            Value::Integer(g, None, _),
            Value::Integer(b, None, _),
            Some(Value::Integer(a, None, _)),
        ) => Some(Color {
            r: *r as _,
            g: *g as _,
            b: *b as _,
            a: *a as _,
        }),
        _ => None,
    }
}

fn value_unit(val: &Value) -> Option<UnitValue> {
    match val {
        Value::Integer(u, Some(Unit::Pixel), _) => Some(UnitValue::Pixels(*u as _)),
        Value::Float(u, Some(Unit::Pixel), _) => Some(UnitValue::Pixels(*u)),
        _ => None,
    }
}

fn rect_form_iter<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Option<UnitRect> {
    let a = value_unit(iter.next()?)?;
    let b = value_unit(iter.next()?)?;
    let c = value_unit(iter.next()?)?;
    let d = value_unit(iter.next()?)?;

    Some(UnitRect::new(a, b, c, d))
}

fn rect_xy_form_iter<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Option<UnitRect> {
    let a = value_unit(iter.next()?)?;
    let b = value_unit(iter.next()?)?;
    Some(UnitRect::new(a, b, a, b))
}

fn rect_all_form_iter<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Option<UnitRect> {
    let a = value_unit(iter.next()?)?;
    Some(UnitRect::new(a, a, a, a))
}

// fn verify_enum()

impl StyleValue {
    fn build_function(key: &str, func: &str, args: &ElementArgs) -> StyleValue {
        match func {
            "rgb" => {
                let Some(color) = color_from_iter(args.iter_values()) else {
                    return StyleValue::Empty
                };

                match key {
                    "foregroundColor" => return StyleValue::ForegroundColor { color },
                    "backgroundColor" => return StyleValue::BackgroundColor { color },
                    "borderColor" => return StyleValue::BorderColor { color },
                    _ => (),
                }
            }
            "rect_xy" => {
                let Some(rect) = rect_xy_form_iter(args.iter_values()) else {
                    return StyleValue::Empty;
                };

                match key {
                    "padding" => return StyleValue::Padding { rect },
                    "radius" => return StyleValue::Radius { rect },
                    "borderWidth" => return StyleValue::BorderWidth { rect },
                    _ => (),
                }
            }
            "rect_all" => {
                let Some(rect) = rect_all_form_iter(args.iter_values()) else {
                    return StyleValue::Empty;
                };

                match key {
                    "padding" => return StyleValue::Padding { rect },
                    "radius" => return StyleValue::Radius { rect },
                    "borderWidth" => return StyleValue::BorderWidth { rect },
                    _ => (),
                }
            }
            "rect" => {
                let Some(rect) = rect_form_iter(args.iter_values()) else {
                    return StyleValue::Empty;
                };

                match key {
                    "padding" => return StyleValue::Padding { rect },
                    "radius" => return StyleValue::Radius { rect },
                    "borderWidth" => return StyleValue::BorderWidth { rect },
                    _ => (),
                }
            }
            _ => (),
        }
        StyleValue::Empty
    }

    pub fn from_symbol(sym: &Node, prop_key: &str) -> StyleValue {
        match &sym.ty {
            NodeType::Style { properties, .. } => {
                if let Some(prop) = properties.get(prop_key) {
                    match prop {
                        Value::Function {
                            ident: Some(SpannedToken(_, Token::Ident(i))),
                            args,
                        } => return StyleValue::build_function(prop_key, i, args),
                        Value::Float(_, _, _) | Value::Integer(_, _, _) => {
                            let Some(uv) = value_unit(prop) else {
                                return StyleValue::Empty
                            };
                            match prop_key {
                                "gap" => return StyleValue::Gap { amount: uv },
                                _ => (),
                            }
                        }
                        Value::Ident(SpannedToken(_, Token::Ident(id))) => {
                            match (prop_key, id.as_str()) {
                                ("direction", "Vertical") => {
                                    return StyleValue::Direction {
                                        direction: Direction::Vertical,
                                    }
                                }
                                ("direction", "Horizontal") => {
                                    return StyleValue::Direction {
                                        direction: Direction::Horizontal,
                                    }
                                }
                                ("direction", "VerticalReverse") => {
                                    return StyleValue::Direction {
                                        direction: Direction::VerticalReverse,
                                    }
                                }
                                ("direction", "HorizontalReverse") => {
                                    return StyleValue::Direction {
                                        direction: Direction::HorizontalReverse,
                                    }
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
        StyleValue::Empty
    }
}

#[macro_export]
macro_rules! StyleValueAs {
  ($e:expr,BackgroundColor) => {
    match$e {
      StyleValue::BackgroundColor {
        color
      } => Some((color)),_ => None,
    }
  };
  ($e:expr,ForegroundColor) => {
    match$e {
      StyleValue::ForegroundColor {
        color
      } => Some((color)),_ => None,
    }
  };
  ($e:expr,BorderWidth) => {
    match$e {
      StyleValue::BorderWidth {
        rect
      } => Some((rect)),_ => None,
    }
  };
  ($e:expr,BorderColor) => {
    match$e {
      StyleValue::BorderColor {
        color
      } => Some((color)),_ => None,
    }
  };
  ($e:expr,Gap) => {
    match$e {
      StyleValue::Gap {
        amount
      } => Some((amount)),_ => None,
    }
  };
  ($e:expr,Padding) => {
    match$e {
      StyleValue::Padding {
        rect
      } => Some((rect)),_ => None,
    }
  };
    ($e:expr,Radius) => {
    match$e {
      StyleValue::Radius {
        rect
      } => Some((rect)),_ => None,
    }
  };
    ($e:expr,Direction) => {
    match$e {
      StyleValue::Direction {
        direction
      } => Some((direction)),_ => None,
    }
  };
}

#[derive(Clone, Copy)]
pub enum UnitValue {
    Pixels(f64),
}

impl Default for UnitValue {
    fn default() -> Self {
        UnitValue::Pixels(0.0)
    }
}

impl Debug for UnitValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for UnitValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitValue::Pixels(u) => write!(f, "{}px", u),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnitRect {
    x0: UnitValue,
    y0: UnitValue,
    x1: UnitValue,
    y1: UnitValue,
}

impl UnitRect {
    pub fn new(x0: UnitValue, y0: UnitValue, x1: UnitValue, y1: UnitValue) -> UnitRect {
        UnitRect { x0, y0, x1, y1 }
    }
}

impl TryInto<Rect> for UnitRect {
    type Error = ();

    fn try_into(self) -> Result<Rect, Self::Error> {
        use UnitValue::*;
        match (self.x0, self.y0, self.x1, self.y1) {
            (Pixels(x0), Pixels(y0), Pixels(x1), Pixels(y1)) => Ok(Rect::new(x0, y0, x1, y1)),
            _ => Err(()),
        }
    }
}

impl TryInto<RoundedRectRadii> for UnitRect {
    type Error = ();

    fn try_into(self) -> Result<RoundedRectRadii, Self::Error> {
        use UnitValue::*;
        match (self.x0, self.y0, self.x1, self.y1) {
            (Pixels(x0), Pixels(y0), Pixels(x1), Pixels(y1)) => {
                Ok(RoundedRectRadii::new(x0, y0, x1, y1))
            }
            _ => Err(()),
        }
    }
}
