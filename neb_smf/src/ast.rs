use std::fmt::Debug;

use neb_util::format::{NodeDisplay, TreeDisplay};

use crate::token::{Range, SpannedToken, Token};

pub trait AstNode: TreeDisplay {
    fn get_range(&self) -> Range;
}

macro_rules! addup {
    ($($e:expr),*) => {{
        $((if let Some(_) = $e { 1 } else { 0 })+)* 0
    }};
}

macro_rules! switchon {
    ($index:expr, $($e:expr),*) => {{
        let mut ind = 0;
        $(if let Some(v) = $e {
            if $index == ind {
                return Some(v)
            }
            ind += 1;
        })*
        ind
    }};
}

// impl<T> TreeDisplay for Option<T>
// where
//     T: TreeDisplay + NodeDisplay,
// {
//     fn num_children(&self) -> usize {
//         match self {
//             Some(s) => s.num_children(),
//             _ => 0,
//         }
//     }

//     fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
//         match self {
//             Some(s) => s.child_at(index),
//             _ => None,
//         }
//     }
//     fn child_at_bx<'a>(&'a self, index: usize) -> Box<dyn TreeDisplay + 'a> {
//         match self {
//             Some(s) => s.child_at_bx(index),
//             _ => panic!(),
//         }
//     }
// }

// impl<T> AstNode for Option<T>
// where
//     T: AstNode + TreeDisplay,
// {
//     fn get_range(&self) -> Range {
//         if let Some(s) = self {
//             s.get_range()
//         } else {
//             Range::default()
//         }
//     }
// }

impl AstNode for SpannedToken {
    fn get_range(&self) -> Range {
        self.0.into()
    }
}

#[derive(Clone)]
pub struct PunctuationList<T: AstNode> {
    tokens: Vec<(T, Option<SpannedToken>)>,
}

impl<T: AstNode> PunctuationList<T> {
    pub fn new() -> PunctuationList<T> {
        PunctuationList { tokens: Vec::new() }
    }

    pub fn push(&mut self, val: T, separator: Option<SpannedToken>) {
        self.tokens.push((val, separator))
    }

    pub fn push_sep(&mut self, val: T, separator: SpannedToken) {
        self.tokens.push((val, Some(separator)))
    }

    pub fn push_term(&mut self, val: T) {
        self.tokens.push((val, None))
    }

    pub fn iter_items(&self) -> impl Iterator<Item = &T> + '_ {
        self.tokens.iter().map(|(v, _)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(T, Option<SpannedToken>)> + '_ {
        self.tokens.iter()
    }
}

impl<T> NodeDisplay for PunctuationList<T>
where
    T: NodeDisplay + AstNode,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Punctuation List")?;
        write!(f, " {}", self.tokens.len())
    }
}

impl<T> TreeDisplay for PunctuationList<T>
where
    T: TreeDisplay + AstNode,
{
    fn num_children(&self) -> usize {
        if let Some((_, Some(_))) = self.tokens.last() {
            self.tokens.len() * 2
        } else if self.tokens.len() > 0 {
            self.tokens.len() * 2 - 1
        } else {
            0
        }
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        let p = &self.tokens[index / 2];
        if index % 2 == 0 {
            Some(&p.0)
        } else {
            Some(p.1.as_ref().unwrap())
        }
    }
}

#[derive(Clone)]
pub struct ElementArgs {
    pub range: Range,
    pub items: PunctuationList<Arg>,
}

impl AstNode for ElementArgs {
    fn get_range(&self) -> Range {
        self.range
    }
}

impl ElementArgs {
    pub fn iter_items(&self) -> impl Iterator<Item = &Arg> + '_ {
        self.items.iter_items()
    }
}

impl NodeDisplay for ElementArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Element Parameters")
    }
}

impl TreeDisplay for ElementArgs {
    fn num_children(&self) -> usize {
        2
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match index {
            0 => Some(&self.range),
            1 => Some(&self.items),
            _ => panic!(),
        }
    }
}

#[derive(Clone)]
pub struct Arg {
    pub name: Option<SpannedToken>,
    pub colon: Option<SpannedToken>,
    pub value: Option<Value>,
}

impl AstNode for Arg {
    fn get_range(&self) -> Range {
        match (&self.name, &self.colon, &self.value) {
            (Some(name), Some(colon), None) => Range::from((name, colon)),
            (Some(name), None, Some(value)) => Range::from((name, &value.get_range())),
            (None, Some(colon), Some(value)) => Range::from((colon, &value.get_range())),
            _ => Range::default(),
        }
    }
}

impl Arg {
    pub fn name(&self) -> &String {
        match &self.name {
            Some(SpannedToken(_, Token::Ident(s))) => s,
            _ => panic!(),
        }
    }
}

impl NodeDisplay for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Parameter")
    }
}

impl TreeDisplay for Arg {
    fn num_children(&self) -> usize {
        addup!(self.name, self.colon, self.value)
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        // match index {
        //     0 => Some(&self.name),
        //     1 => Some(&self.colon),
        //     2 => Some(&self.value),
        //     _ => panic!(),
        // }

        switchon!(index, &self.name, &self.colon, &self.value);
        None
    }
}

pub enum Expression {
    Ident(SpannedToken),
}

impl AstNode for Expression {
    fn get_range(&self) -> Range {
        match self {
            Self::Ident(i) => i.0.into(),
        }
    }
}

impl NodeDisplay for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Ident(s) => write!(f, "{}", s.format()),
        }
    }
}

impl TreeDisplay for Expression {
    fn num_children(&self) -> usize {
        match self {
            Self::Ident(_) => 0,
        }
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        match self {
            Self::Ident(_) => None,
        }
    }
}

#[derive(Clone)]
pub enum Value {
    Integer(u64, SpannedToken),
    Float(f64, SpannedToken),
    Ident(SpannedToken),
    Function {
        ident: Option<SpannedToken>,
        args: ElementArgs,
    },
    Tuple(Vec<Value>),
}

impl AstNode for Value {
    fn get_range(&self) -> Range {
        match self {
            Self::Tuple(s) => match (s.first(), s.last()) {
                (Some(s), Some(e)) => Range::from((&s.get_range(), &e.get_range())),
                _ => Range::default(),
            },
            Self::Integer(_, s) => s.0.into(),
            Self::Float(_, s) => s.0.into(),
            Self::Ident(s) => s.0.into(),
            Self::Function { ident: None, args } => args.get_range(),
            Self::Function {
                ident: Some(ident),
                args,
            } => Range::from((ident, &args.get_range())),
        }
    }
}

impl NodeDisplay for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Integer(i, _) => write!(f, "{}", i),
            Self::Float(i, _) => write!(f, "{}", i),
            Self::Ident(SpannedToken(_, Token::Ident(i))) => write!(f, "{}", i),
            Self::Function {
                ident: Some(SpannedToken(_, Token::Ident(i))),
                ..
            } => write!(f, "Function {}", i),
            Self::Function { ident: None, .. } => write!(f, "Function"),
            _ => panic!(),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Value as NodeDisplay>::fmt(&self, f)
    }
}

impl TreeDisplay for Value {
    fn num_children(&self) -> usize {
        match self {
            Self::Function { .. } => 1,
            _ => 0,
        }
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        match self {
            Self::Function { args, .. } => Some(args),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub enum StyleStatement {
    StyleElement {
        key: Option<SpannedToken>,
        colon: Option<SpannedToken>,
        value: Option<Value>,
    },
    Style {
        body: Vec<StyleStatement>,
        body_range: Option<Range>,
        token: Option<SpannedToken>,
    },
}

impl AstNode for StyleStatement {
    fn get_range(&self) -> Range {
        match self {
            Self::Style {
                body_range: Some(body_range),
                token: Some(token),
                ..
            } => Range::from((token, body_range)),
            Self::Style {
                body_range: None,
                token: Some(token),
                ..
            } => Range::from(token.0),
            Self::Style {
                body_range: Some(body_range),
                token: None,
                ..
            } => body_range.clone(),
            _ => Range::default(),
        }
    }
}

impl NodeDisplay for StyleStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Style Statement")
    }
}

impl TreeDisplay for StyleStatement {
    fn num_children(&self) -> usize {
        match self {
            Self::StyleElement {
                key,
                colon: _,
                value,
            } => addup!(key, value),
            Self::Style {
                body_range,
                token,
                body,
            } => addup!(body_range, token) + body.len(),
        }
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match self {
            Self::StyleElement {
                key,
                colon: _,
                value,
            } => {
                switchon!(index, key, value);
                None
            }
            Self::Style {
                body,
                body_range,
                token,
                ..
            } => {
                let ind = switchon!(index, token, body_range);
                Some(&body[index - ind])
            }
        }
    }
}

pub enum Statement {
    // Expression(Expression),
    UseStatement {
        token: Option<SpannedToken>,
        args: PunctuationList<SpannedToken>,
    },
    Element {
        arguments: Option<ElementArgs>,
        body: Vec<Statement>,
        body_range: Option<Range>,
        token: Option<SpannedToken>,
    },
    Style {
        body: Vec<StyleStatement>,
        body_range: Option<Range>,
        token: Option<SpannedToken>,
    },
}

impl AstNode for Statement {
    fn get_range(&self) -> Range {
        match self {
            // Self::Expression(e) => e.get_range(),
            Self::Element {
                body_range: Some(body_range),
                token: Some(token),
                ..
            } => Range::from((token, body_range)),
            Self::Element {
                body_range: Some(body_range),
                arguments: Some(arguments),
                token: None,
                ..
            } => Range::from((&arguments.get_range(), body_range)),
            Self::Element {
                body_range: Some(body_range),
                arguments: None,
                token: None,
                ..
            } => body_range.clone(),
            Self::Style {
                body_range: Some(body_range),
                token: Some(token),
                ..
            } => Range::from((token, body_range)),
            Self::Style {
                body_range: None,
                token: Some(token),
                ..
            } => Range::from(token.0),
            Self::Style {
                body_range: Some(body_range),
                token: None,
                ..
            } => body_range.clone(),
            _ => Range::default(),
        }
    }
}

impl NodeDisplay for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Statement")
        // match self {
        //     Self::Element { .. } => f.write_str("Element"),
        //     Self::Expression { .. } => f.write_str("Expression"),
        // }
    }
}

impl TreeDisplay for Statement {
    fn num_children(&self) -> usize {
        match self {
            Self::Element {
                arguments,
                body_range,
                token,
                body,
            } => addup!(arguments, body_range, token) + body.len(),
            Self::Style {
                body_range,
                token,
                body,
            } => addup!(body_range, token) + body.len(),
            Self::UseStatement { token, args } => addup!(token) + args.num_children(), // Self::Expression(_) => 1,
        }
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match self {
            Self::Element {
                body,
                body_range,
                token,
                arguments,
                ..
            } => {
                let ind = switchon!(index, token, arguments, body_range);
                Some(&body[index - ind])
            }
            Self::Style {
                body,
                body_range,
                token,
                ..
            } => {
                let ind = switchon!(index, token, body_range);
                Some(&body[index - ind])
            }
            Self::UseStatement { token, args } => {
                let ind = switchon!(index, token);
                args.child_at(index - ind)
            }
        }
    }

    fn child_at_bx<'b>(&'b self, index: usize) -> Box<dyn TreeDisplay + 'b> {
        match self {
            Self::Element {
                token: Some(SpannedToken(_, Token::Ident(name))),
                ..
            } => match index {
                0 => Box::new(format!("Name: `{}`", name)),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
}
