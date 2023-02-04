use std::collections::HashSet;
use std::fmt::{Debug, Display};

use neb_graphics::vello::kurbo::{Rect, RoundedRectRadii};
use neb_graphics::vello::peniko::Color;
use neb_macros::EnumHash;
use neb_smf::ast::Value;
use neb_smf::{Symbol, SymbolKind};

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
    match (r, g, b) {
        (Value::Integer(r, _), Value::Integer(g, _), Value::Integer(b, _)) => Some(Color {
            r: *r as _,
            g: *g as _,
            b: *b as _,
            a: 255,
        }),
        _ => None,
    }
}

impl StyleValue {
    pub fn from_symbol(sym: &Symbol, prop_key: &str) -> StyleValue {
        match &sym.kind {
            SymbolKind::Style { properties } => {
                if let Some(prop) = properties.get(prop_key) {
                    let func = prop.as_function();
                    match (prop_key, func) {
                        (
                            "backgroundColor" | "foregroundColor" | "borderColor",
                            Some(("rgb", args)),
                        ) => {
                            if let Some(color) = color_from_iter(args.iter_values()) {
                                return StyleValue::BackgroundColor { color };
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

/*
#[derive(Debug)]
pub struct Selector {
    // element: Cow<'a, str>,
    element: String,
    values: HashMap<String, StyleValue>,
}

impl Selector {
    // pub fn hashes(&self) -> impl Iterator<Item = (u64, &StyleValue)> + '_ {
    //     self.values.iter().map(|f| (calculate_hash(f.0), f.1))
    // }
    pub fn values(&self) -> Values<String, StyleValue> {
        self.values.values()
    }

    pub fn get(&self, key: &str) -> Option<&StyleValue> {
        self.values.get(key)
    }

    pub fn map(&self) -> &HashMap<String, StyleValue> {
        &self.values
    }
}

pub fn parse_styles(
    input: &str,
) -> Result<HashMap<String, Rf<Selector>>, nom::Err<nom::error::Error<&[u8]>>> {
    let mut map = HashMap::new();

    let (_, styles) = parse_styles_impl(input.as_bytes())?;
    dbg!(&styles);
    for style in styles {
        map.insert(style.element.clone(), Rf::new(style));
    }

    Ok(map)
}

pub fn parse_styles_impl(bytes: &[u8]) -> IResult<&[u8], Vec<Selector>> {
    many0(map(pair(alpha1, parse_block), |val| {
        let key = std::str::from_utf8(val.0);
        let key = key.unwrap().to_owned();
        Selector {
            element: key,
            values: val.1,
        }
    }))(bytes)
}

pub fn parse_block(bytes: &[u8]) -> IResult<&[u8], HashMap<String, StyleValue>> {
    map(
        delimited(
            wa(tag("{")),
            separated_list0(wal(tag("\n")), parse_value),
            wa(tag("}")),
        ),
        |vals| {
            let mut hm = HashMap::new();
            for val in vals {
                hm.insert(val.0, val.1);
            }
            hm
            // vals.into_iter().map(|f| f.1).collect()
        },
    )(bytes)
}

pub fn parse_value(bytes: &[u8]) -> IResult<&[u8], (String, StyleValue)> {
    let (val, (key, _)) = separated_pair(
        alpha1,
        wal(tag(":")),
        take_while(is_none_of::<&[u8], &str>("\n")),
    )(bytes)?;

    let st = std::str::from_utf8(key).unwrap();

    let hash = calculate_hash(st);

    dbg!(st, hash);
    let (bytes, style) = match hash {
        StyleValueHashes::BackgroundColor => {
            let (bytes, color) = parse_color(val)?;
            (bytes, StyleValue::BackgroundColor { color })
        }
        StyleValueHashes::Gap => {
            let (bytes, value) = parse_units_values(val)?;
            (bytes, StyleValue::Gap { amount: value })
        }
        StyleValueHashes::Padding => {
            let (bytes, value) = parse_rect(val)?;
            (bytes, StyleValue::Padding { rect: value })
        }
        StyleValueHashes::BorderColor => {
            let (bytes, value) = parse_color(val)?;
            (bytes, StyleValue::BorderColor { color: value })
        }
        StyleValueHashes::BorderWidth => {
            let (bytes, value) = parse_rect(val)?;
            (bytes, StyleValue::BorderWidth { rect: value })
        }
        StyleValueHashes::ForegroundColor => {
            let (bytes, value) = parse_color(val)?;
            (bytes, StyleValue::ForegroundColor { color: value })
        }
        StyleValueHashes::Radius => {
            let (bytes, value) = parse_rect(val)?;
            (bytes, StyleValue::Radius { rect: value })
        }
        StyleValueHashes::Direction => {
            let (bytes, value) = parse_direction(val)?;
            (bytes, StyleValue::Direction { direction: value })
        }
        _ => panic!(),
    };

    Ok((bytes, (st.into(), style)))
}

pub fn parse_color(bytes: &[u8]) -> IResult<&[u8], Color> {
    alt((parse_rgb, parse_hex))(bytes)
}

pub fn parse_rgb(bytes: &[u8]) -> IResult<&[u8], Color> {
    preceded(
        tag("rgb"),
        delimited(
            wal(tag("(")),
            map(separated_list1(wal(tag(",")), digit1), |val| {
                let r = std::str::from_utf8(val[0]).unwrap().parse::<u8>().unwrap();
                let g = std::str::from_utf8(val[1]).unwrap().parse::<u8>().unwrap();
                let b = std::str::from_utf8(val[2]).unwrap().parse::<u8>().unwrap();
                Color::rgb8(r, g, b)
            }),
            wal(tag(")")),
        ),
    )(bytes)
}

pub fn parse_hex(bytes: &[u8]) -> IResult<&[u8], Color> {
    preceded(
        tag("#"),
        map_res(
            hex_digit1,
            |digit| {
                let str = std::str::from_utf8(digit).unwrap();
                if str.len() == 3 {
                    let mut it = str.chars();
                    let r = char_to_hex(it.next().unwrap());
                    let g = char_to_hex(it.next().unwrap());
                    let b = char_to_hex(it.next().unwrap());
                    Ok(Color::rgb8(r, g, b))
                } else if str.len() == 6 {
                    let mut it = str.chars();

                    let r1 = it.next().unwrap();
                    let r2 = it.next().unwrap();
                    let p = char_to_hex(r1);
                    let r = p << 4 | char_to_hex(r2);

                    let g1 = it.next().unwrap();
                    let g2 = it.next().unwrap();
                    let g = char_to_hex(g1) << 4 | char_to_hex(g2);

                    let b1 = it.next().unwrap();
                    let b2 = it.next().unwrap();
                    let b = char_to_hex(b1) << 4 | char_to_hex(b2);

                    Ok(Color::rgb8(r, g, b))
                } else {
                    Err(nom::Err::Error(nom::error::Error::new(
                        bytes,
                        nom::error::ErrorKind::Char,
                    )))
                }
            }, // pair(

               //     one_of("0123456789abcdefABCDEF"),
               //     one_of("0123456789abcdefABCDEF"),
               // ),
               // |(a, b)| {
               //     let hi = char_to_hex(a);
               //     let lo = char_to_hex(b);

               //     (hi << 4) & lo
               // },
        ),
    )(bytes)
}

pub fn parse_direction(bytes: &[u8]) -> IResult<&[u8], Direction> {
    map(
        alt((
            tag("Vertical"),
            tag("VerticalReverse"),
            tag("Horizontal"),
            tag("HorizontalReverse"),
        )),
        |val| match std::str::from_utf8(val).unwrap() {
            "Vertical" => Direction::Vertical,
            "VerticalReverse" => Direction::VerticalReverse,
            "Horizontal" => Direction::Horizontal,
            "HorizontalReverse" => Direction::HorizontalReverse,
            _ => panic!(),
        },
    )(bytes)
}

pub fn parse_rect(bytes: &[u8]) -> IResult<&[u8], UnitRect> {
    map_res(
        separated_list1(wal(tag(",")), parse_units_values),
        |vals| match vals.len() {
            1 => Ok(UnitRect::new(vals[0], vals[0], vals[0], vals[0])),
            2 => Ok(UnitRect::new(vals[0], vals[1], vals[0], vals[1])),
            4 => Ok(UnitRect::new(vals[0], vals[1], vals[2], vals[3])),
            _ => Err(nom::Err::Error(nom::error::Error::new(
                bytes,
                nom::error::ErrorKind::SeparatedList,
            ))),
        },
    )(bytes)
}



pub fn parse_units_values(bytes: &[u8]) -> IResult<&[u8], UnitValue> {
    map(terminated(recognize_float, tag("px")), |val| {
        UnitValue::Pixels(std::str::from_utf8(val).unwrap().parse::<f64>().unwrap())
    })(bytes)
}

fn calculate_hash<T>(t: &T) -> u64
where
    T: Hash + ?Sized,
{
    let mut state = DefaultHasher::new();
    t.hash(&mut state);
    state.finish()
}

fn char_to_hex(c: char) -> u8 {
    let c = c.to_ascii_lowercase() as u8;
    if c >= b'a' {
        c - b'a' + 9
    } else {
        c - b'0'
    }
}

pub fn is_none_of<I, T>(list: T) -> impl Fn(<I as InputTakeAtPosition>::Item) -> bool
where
    I: InputTakeAtPosition,
    <I as InputTakeAtPosition>::Item: AsChar + Copy,
    T: FindToken<<I as InputTakeAtPosition>::Item>,
{
    move |c| list.find_token(c)
}

pub fn ws<I, E: ParseError<I>>() -> impl FnMut(I) -> IResult<I, I, E>
where
    I: InputTakeAtPosition,
    <I as InputTakeAtPosition>::Item: AsChar + Copy,
{
    take_while(|c: <I as InputTakeAtPosition>::Item| {
        match <<I as InputTakeAtPosition>::Item as AsChar>::as_char(c) {
            ' ' | '\t' | '\n' | '\r' => true,
            _ => false,
        }
    })
}

pub fn wa<I, O1, E: ParseError<I>, F>(mut first: F) -> impl FnMut(I) -> IResult<I, O1, E>
where
    I: InputTakeAtPosition,
    <I as InputTakeAtPosition>::Item: AsChar + Copy,
    F: nom::Parser<I, O1, E>,
{
    move |input: I| {
        let (input, _) = nom::Parser::parse(&mut ws(), input)?;
        let (input, o1) = first.parse(input)?;
        nom::Parser::parse(&mut ws(), input).map(|(i, _)| (i, o1))
    }
}

pub fn wsl<I, E: ParseError<I>>() -> impl FnMut(I) -> IResult<I, I, E>
where
    I: InputTakeAtPosition,
    <I as InputTakeAtPosition>::Item: AsChar + Copy,
{
    take_while(|c: <I as InputTakeAtPosition>::Item| {
        match <<I as InputTakeAtPosition>::Item as AsChar>::as_char(c) {
            ' ' | '\t' => true,
            _ => false,
        }
    })
}

pub fn wal<I, O1, E: ParseError<I>, F>(mut first: F) -> impl FnMut(I) -> IResult<I, O1, E>
where
    I: InputTakeAtPosition,
    <I as InputTakeAtPosition>::Item: AsChar + Copy,
    F: nom::Parser<I, O1, E>,
{
    move |input: I| {
        let (input, _) = nom::Parser::parse(&mut wsl(), input)?;
        let (input, o1) = first.parse(input)?;
        nom::Parser::parse(&mut wsl(), input).map(|(i, _)| (i, o1))
    }
}
 */
