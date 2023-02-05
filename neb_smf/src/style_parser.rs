use crate::{
    ast::{StyleStatement, Value},
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

        let value = self.parse_value();

        Some(StyleStatement::StyleElement {
            key: key.cloned(),
            colon: colon.cloned(),
            value,
        })
    }

    pub fn parse_value(&self) -> Option<Value> {
        match self.tokens.peek() {
            Some(Token::Integer(i, u)) => {
                Some(Value::Integer(*i, *u, self.tokens.next().cloned().unwrap()))
            }
            Some(Token::Float(i, u)) => {
                Some(Value::Float(*i, *u, self.tokens.next().cloned().unwrap()))
            }
            Some(Token::Ident(_)) => {
                let ident = self.tokens.next().unwrap();

                if let Some(Token::Operator(Operator::OpenParen)) = self.tokens.peek() {
                    return Some(Value::Function {
                        ident: Some(ident.clone()),
                        args: self.parse_args().unwrap(),
                    });
                } else {
                    Some(Value::Ident(ident.clone()))
                }
            }
            _ => None,
        }
    }

    // fn parse_style_args(&self) -> Vec<Value> {
    //     let open_paren= self.expect_operator(Operator::OpenParen);

    //     let close_paren= self.expect_operator(Operator::CloseParen);
    // }
}
