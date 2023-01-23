use std::sync::RwLock;

use neb_util::format::{NodeDisplay, TreeDisplay};


#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Colon,
    Comma,
}

impl Operator {
    pub fn as_str(&self) -> &str {
        match self {
            Self::OpenParen => "`(`",
            Self::CloseParen => "`)`",
            Self::OpenBrace => "`{`",
            Self::CloseBrace => "`}`",
            Self::Colon => "`:`",
            Self::Comma => "`,`",
        }
    }
}

#[derive(Debug)]
pub enum Keyword {
    // Output,
}

#[derive(Debug, Clone)]
pub enum Token {
    Ident(String),
    Integer(u64),
    Float(f64),
    Operator(Operator),

    // Keyword(Keyword),
    Newline,
    Whitespace,
}

impl NodeDisplay for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(s) => f.write_str(s),
            Self::Operator(o) => f.write_str(o.as_str()),
            Self::Integer(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::Newline => f.write_str("Newline"),
            Self::Whitespace => f.write_str("Whitespace"),
        }
    }
}

#[derive(Debug)]
pub struct TokenStream {
    tokens: Vec<SpannedToken>,
    next_index: RwLock<usize>,
}

impl<'a> TokenStream {
    pub fn next(&self) -> Option<&SpannedToken> {
        let next_index = *self.next_index.read().unwrap();
        if next_index >= self.tokens.len() {
            return None;
        }
        let r = &self.tokens[next_index];
        let mut s = self.next_index.write().unwrap();
        *s += 1;
        Some(r)
    }

    pub fn peek(&'a self) -> Option<&'a Token> {
        let next_index = *self.next_index.read().unwrap();
        if next_index >= self.tokens.len() {
            return None;
        }
        Some(&self.tokens[next_index].tok())
    }
}

// impl<'a> From<Vec<Token<'a>>> for TokenStream<'a> {
//     fn from(value: Vec<Token<'a>>) -> Self {
//         TokenStream {
//             tokens: value,
//             next_index: RwLock::new(0),
//         }
//     }
// }

impl<'a> From<Vec<SpannedToken>> for TokenStream {
    fn from(value: Vec<SpannedToken>) -> Self {
        TokenStream {
            tokens: value,
            next_index: RwLock::new(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpannedToken(pub Span, pub Token);

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self(span, token)
    }

    pub fn tok(&self) -> &Token {
        &self.1
    }

    pub fn span(&self) -> &Span {
        &self.0
    }
}

impl<'a> NodeDisplay for SpannedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token: ",)?;
        self.1.fmt(f)
    }
}

impl<'a> TreeDisplay for SpannedToken {
    fn num_children(&self) -> usize {
        1
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        Some(&self.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub line_num: u32,
    pub position: u32,
    pub length: u32,
    pub token_index: u32,
}

impl Span {
    pub fn contains(&self, other: &Span) -> bool {
        if self.line_num == other.line_num {
            if other.position < self.position + self.length {
                return true;
            }
        }
        false
    }

    pub fn before(&self, other: &Span) -> bool {
        if self.line_num == other.line_num {
            if other.position >= self.position + self.length {
                return true;
            }
        }
        false
    }

    pub fn right_before(&self, other: &Span) -> bool {
        if self.line_num == other.line_num {
            if other.position == self.position + self.length {
                return true;
            }
        }
        false
    }

    pub fn after(&self, other: &Span) -> bool {
        if self.line_num == other.line_num {
            if other.position + other.length < self.position {
                return true;
            }
        }
        false
    }

    pub fn right_after(&self, other: &Span) -> bool {
        if self.line_num == other.line_num {
            if other.position + other.length == self.position {
                return true;
            }
        }
        false
    }
}

impl From<SpannedToken> for Span {
    fn from(value: SpannedToken) -> Self {
        value.0
    }
}

impl From<&SpannedToken> for Span {
    fn from(value: &SpannedToken) -> Self {
        value.0
    }
}

impl Ord for Span {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.line_num.partial_cmp(&other.line_num) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.position.partial_cmp(&other.position)
    }
}

impl NodeDisplay for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Span line: {}, character: {}, {} long, token: {}",
            self.line_num, self.position, self.length, self.token_index
        )
    }
}

impl TreeDisplay for Span {
    fn num_children(&self) -> usize {
        0
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        panic!()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Range {
    pub start: Span,
    pub end: Span,
}

impl Range {
    pub fn new(start: Span, end: Span) -> Range {
        Range { start, end }
    }

    pub fn contains(&self, span: &Span) -> bool {
        span >= &self.start && span <= &self.end
    }
}

impl From<(&Range, &Range)> for Range {
    fn from(value: (&Range, &Range)) -> Self {
        Range {
            start: value.0.start,
            end: value.1.end,
        }
    }
}

impl<T> From<(&Range, T)> for Range
where
    T: Into<Span>,
{
    fn from(value: (&Range, T)) -> Self {
        Range {
            start: value.0.start,
            end: value.1.into(),
        }
    }
}

impl<T> From<(T, &Range)> for Range
where
    T: Into<Span>,
{
    fn from(value: (T, &Range)) -> Self {
        Range {
            start: value.0.into(),
            end: value.1.end,
        }
    }
}

impl<T, U> From<(T, U)> for Range
where
    T: Into<Span>,
    U: Into<Span>,
{
    fn from(value: (T, U)) -> Self {
        Range {
            start: value.0.into(),
            end: value.1.into(),
        }
    }
}

impl From<Span> for Range {
    fn from(value: Span) -> Self {
        Range {
            start: value,
            end: value,
        }
    }
}

impl NodeDisplay for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Range")
    }
}

impl TreeDisplay for Range {
    fn num_children(&self) -> usize {
        2
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match index {
            0 => Some(&self.start),
            1 => Some(&self.end),
            _ => panic!(),
        }
    }
}
