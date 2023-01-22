use crate::{
    ast::{StyleStatement, StyleValue},
    parser::Parser,
    token::{Operator, Range, Token},
};

impl Parser {
    pub fn parse_style_statement(&self) -> Option<StyleStatement> {
        let ident = match self.tokens.peek() {
            Some(Token::Ident(_)) => self.tokens.next(),
            _ => None,
        };

        let open_brace = self.expect_operator(Operator::OpenBrace);
        let mut statements = Vec::new();

        while let Some(statement) = self.parse_style_element() {
            statements.push(statement);
            if let Some(Token::Operator(Operator::CloseBrace)) = self.tokens.peek() {
                break;
            }
        }

        let close_brace = self.expect_operator(Operator::CloseBrace);

        Some(StyleStatement::Style {
            body: statements,
            body_range: open_brace.zip(close_brace).map(|(o, c)| Range {
                start: o.span().clone(),
                end: c.span().clone(),
            }),
            token: ident.cloned(),
        })
    }

    fn parse_style_element(&self) -> Option<StyleStatement> {
        let key = match self.tokens.peek() {
            Some(Token::Ident(_)) => self.tokens.next(),
            _ => None,
        };

        let colon = self.expect_operator(Operator::Colon);

        let value = self.parse_style_value();

        Some(StyleStatement::StyleElement {
            key: key.cloned(),
            colon: colon.cloned(),
            value,
        })
    }

    fn parse_style_value(&self) -> Option<StyleValue> {
        match self.tokens.peek() {
            Some(Token::Integer(i)) => Some(StyleValue::Integer(
                *i,
                self.tokens.next().cloned().unwrap(),
            )),
            Some(Token::Float(i)) => {
                Some(StyleValue::Float(*i, self.tokens.next().cloned().unwrap()))
            }
            Some(Token::Ident(i)) => Some(StyleValue::Ident(self.tokens.next().cloned().unwrap())),
            _ => None,
        }
    }
}
