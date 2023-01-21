use std::sync::RwLock;

use crate::format::{NodeDisplay, TreeDisplay};

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

#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
    pub line_num: u32,
    pub position: u32,
    pub length: u32,
    pub token_index: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Range {
    pub start: Span,
    pub end: Span,
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
