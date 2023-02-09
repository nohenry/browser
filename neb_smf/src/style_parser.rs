use crate::{
    ast::{PunctuationList, StyleStatement, Value},
    error::{ParseError, ParseErrorKind},
    parser::Parser,
    token::{Operator, Range, SpannedToken, Token},
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
            Some(Token::Ident(_)) => self.tokens.next().cloned(),
            Some(Token::Text(_)) => {
                if let Some(SpannedToken(span, Token::Text(i))) = self.tokens.next() {
                    Some(SpannedToken::new(Token::Ident(i.clone()), span.clone()))
                } else {
                    None
                }
            }
            _ => None,
        };

        let colon = self.expect_operator(Operator::Colon);

        let value = self.parse_value();

        Some(StyleStatement::StyleElement {
            key: key,
            colon: colon.cloned(),
            value,
        })
    }

    pub fn parse_array(&self) -> Option<Value> {
        let open = self.expect_operator(Operator::OpenSquare);

        let args = match self.tokens.peek() {
            Some(Token::Operator(Operator::CloseParen)) => PunctuationList::new(),
            _ => {
                let mut args = PunctuationList::new();

                while let Some(arg) = self.parse_value() {
                    let comma = if let Some(Token::Operator(Operator::Comma)) = self.tokens.peek() {
                        self.tokens.next().cloned()
                    } else {
                        None
                    };
                    if let Some(Token::Operator(Operator::CloseSquare)) = self.tokens.peek() {
                        args.push(arg, comma);
                        break;
                    }
                    if comma.is_none() {
                        self.add_error(ParseError {
                            kind: ParseErrorKind::InvalidSyntax(format!(
                                "Expected comma in arguments!"
                            )),
                            range: Range::default(),
                        });
                    }
                    args.push_sep(arg, comma.unwrap());
                }
                args
            }
        };

        let close = self.expect_operator(Operator::CloseSquare);

        if let (Some(open), Some(close)) = (open, close) {
            Some(Value::Array {
                values: args,
                range: Range::from((open.0, close.0)),
            })
        } else {
            self.add_error(ParseError {
                kind: ParseErrorKind::InvalidSyntax(format!("Unable to parse arg brackets!")),
                range: Range::default(),
            });
            Some(Value::Array {
                values: args,
                range: Range::default(),
            })
        }
    }

    pub fn parse_value(&self) -> Option<Value> {
        match self.tokens.peek() {
            Some(Token::Operator(Operator::OpenSquare)) => self.parse_array(),
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
