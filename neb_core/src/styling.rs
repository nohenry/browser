use std::collections::hash_map::Values;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::Hasher,
};

use neb_graphics::vello::peniko::Color;
use neb_macros::EnumHash;
use nom::branch::alt;
use nom::bytes::streaming::take_while;
use nom::character::complete::one_of;
use nom::character::streaming::digit1;
use nom::error::ParseError;
use nom::multi::count;
use nom::number::streaming::recognize_float;
use nom::sequence::{pair, preceded, terminated};
use nom::{
    bytes::complete::tag,
    character::complete::alpha1,
    combinator::map,
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, separated_pair},
    IResult,
};
use nom::{AsChar, FindToken, InputTakeAtPosition};

use crate::Rf;

#[derive(EnumHash, Debug)]
pub enum StyleValue {
    BackgroundColor { color: Color },
    Gap { amount: UnitValue },
}

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
    let (_, val) = preceded(
        tag("#"),
        count(
            map(
                pair(
                    one_of("0123456789abcdefABCDEF"),
                    one_of("0123456789abcdefABCDEF"),
                ),
                |(a, b)| {
                    let hi = char_to_hex(a);
                    let lo = char_to_hex(b);

                    (hi << 4) & lo
                },
            ),
            3,
        ),
    )(bytes)?;

    Ok((bytes, Color::rgb8(val[0], val[1], val[2])))
}

#[derive(Clone, Copy)]
pub enum UnitValue {
    Pixels(f32),
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

pub fn parse_units_values(bytes: &[u8]) -> IResult<&[u8], UnitValue> {
    map(terminated(recognize_float, tag("px")), |val| {
        UnitValue::Pixels(std::str::from_utf8(val).unwrap().parse::<f32>().unwrap())
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
        c - b'a'
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
