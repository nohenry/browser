use std::sync::{RwLock, RwLockReadGuard};

use crate::{
    ast::{Arg, ElementArgs, PunctuationList, Statement},
    error::{ParseError, ParseErrorKind},
    token::{Operator, Range, SpannedToken, Token, TokenStream},
};

pub struct Parser {
    pub(crate) tokens: TokenStream,
    pub(crate) errors: RwLock<Vec<ParseError>>,
}

impl Parser {
    pub fn new(token_stream: impl Into<TokenStream>) -> Self {
        Self {
            tokens: token_stream.into(),
            errors: RwLock::new(Vec::new()),
        }
    }

    pub fn get_errors(&self) -> RwLockReadGuard<'_, Vec<ParseError>> {
        self.errors.read().unwrap()
    }

    pub fn add_error(&self, error: ParseError) {
        let p = &mut *self.errors.write().unwrap();
        p.push(error);
    }

    pub fn parse(&self) -> Option<Vec<Statement>> {
        let mut statements = Vec::new();
        self.ignore_ws();
        while let Some(stmt) = self.parse_statement(false) {
            statements.push(stmt);

            if let Some(Token::Newline) = self.tokens.peek() {
                self.tokens.next();
            }
            self.ignore_ws();
        }

        Some(statements)
    }

    pub fn parse_statement(&self, in_view: bool) -> Option<Statement> {
        let tok = match self.tokens.peek() {
            Some(Token::Ident(s)) if s == "use" => {
                if let Some(us) = self.parse_use() {
                    return Some(us);
                } else {
                    None
                }
            }
            Some(Token::Ident(_)) => self.tokens.next(),
            Some(Token::Text(_)) if in_view => {
                let Some(tok) = self.tokens.next() else {
                    return None;
                };

                return Some(Statement::Text(tok.clone()));
            }
            Some(Token::Text(_)) => self.tokens.next(),
            _ => None,
        };

        match self.tokens.peek() {
            Some(_) => return self.parse_element(tok),
            _ => return None,
        }
    }

    pub fn parse_use(&self) -> Option<Statement> {
        let token = self.tokens.next();
        let mut args = PunctuationList::new();
        let mut last_line = token.map(|l| l.span().line_num);
        while let Some(Token::Ident(_)) = self.tokens.peek() {
            let tok = self.tokens.next();

            match (self.tokens.peek(), tok) {
                (Some(Token::Operator(Operator::Dot)), Some(id)) => {
                    let dot = self.tokens.next();
                    args.push(id.clone(), dot.cloned());
                }
                (_, Some(id)) => {
                    let lline = *last_line.get_or_insert(id.span().line_num);
                    if lline == id.span().line_num {
                        args.push(id.clone(), None);
                    } else {
                        self.tokens.back();
                    }
                    break;
                }
                _ => break,
            }
        }
        Some(Statement::UseStatement {
            token: token.cloned(),
            args,
        })
    }

    pub fn parse_element(&self, ident: Option<&SpannedToken>) -> Option<Statement> {
        let args = if let Some(Token::Operator(Operator::OpenParen)) = self.tokens.peek() {
            self.parse_args()
        } else {
            None
        };

        let open_brace = self.expect_operator(Operator::OpenBrace);
        let statements = if let Some(Token::Operator(Operator::CloseBrace)) = self.tokens.peek() {
            vec![]
        } else {
            match ident {
                Some(SpannedToken(_, Token::Ident(i))) if &i == &"style" => {
                    let mut statements = Vec::new();
                    while let Some(stmt) = self.parse_style_statement() {
                        statements.push(stmt);
                        if let Some(Token::Operator(Operator::CloseBrace)) = self.tokens.peek() {
                            let close_brace = self.tokens.next();

                            return Some(Statement::Style {
                                body: statements,
                                body_range: open_brace.zip(close_brace).map(|(o, c)| Range {
                                    start: o.span().clone(),
                                    end: c.span().clone(),
                                }),
                                token: ident.cloned(),
                            });
                        }
                    }
                    vec![]
                }
                _ => {
                    let view = if let Some(SpannedToken(_, Token::Ident(s))) = ident {
                        s == "view"
                    } else {
                        false
                    };
                    let mut statements = Vec::new();
                    while let Some(stmt) = self.parse_statement(view) {
                        statements.push(stmt);
                        if let Some(Token::Operator(Operator::CloseBrace)) = self.tokens.peek() {
                            break;
                        }
                    }
                    statements
                }
            }
        };

        let close_brace = self.tokens.next();

        Some(Statement::Element {
            arguments: args,
            body: statements,
            body_range: open_brace.zip(close_brace).map(|(o, c)| Range {
                start: o.span().clone(),
                end: c.span().clone(),
            }),
            token: ident.cloned(),
        })

        // if let (Some(open), Some(close), Some(st), Some(ident)) =
        //     (open_brace, close_brace, st, ident)
        // {
        //     Some(Statement::Element {
        //         arguments: args,
        //         body: statements,
        //         body_range: Some(Range {
        //             start: *open.span(),
        //             end: *close.span(),
        //         },
        //         token: ident.clone(),
        //     }))
        // } else {
        //     Some(Statement::PartialElement {
        //         e: vec![
        //             Box::new(open_brace.cloned()),
        //             Box::new(close_brace.cloned()),
        //             Box::new(ident.cloned()),
        //         ],
        //     })
        // }
    }


    pub fn parse_args(&self) -> Option<ElementArgs> {
        let open = self.expect_operator(Operator::OpenParen);

        let args = match self.tokens.peek() {
            Some(Token::Operator(Operator::CloseParen)) => PunctuationList::new(),
            _ => {
                let mut args = PunctuationList::new();

                while let Some(arg) = self.parse_arg() {
                    let comma = if let Some(Token::Operator(Operator::Comma)) = self.tokens.peek() {
                        self.tokens.next().cloned()
                    } else {
                        None
                    };
                    if let Some(Token::Operator(Operator::CloseParen)) = self.tokens.peek() {
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

        let close = self.expect_operator(Operator::CloseParen);

        if let (Some(open), Some(close)) = (open, close) {
            Some(ElementArgs {
                items: args,
                range: Range {
                    start: open.0,
                    end: close.0,
                },
            })
        } else {
            self.add_error(ParseError {
                kind: ParseErrorKind::InvalidSyntax(format!("Unable to parse arg brackets!")),
                range: Range::default(),
            });
            Some(ElementArgs {
                items: args,
                range: Range::default(),
            })
        }
    }

    fn parse_arg(&self) -> Option<Arg> {
        let ident = self.expect(Token::Ident("".into()));
        let colon = self.expect_operator(Operator::Colon);
        let expression = self.parse_value();

        match (ident, colon, expression) {
            (Some(ident), Some(colon), Some(expr)) => Some(Arg {
                name: Some(ident.clone()),
                colon: Some(colon.clone()),
                value: Some(expr),
            }),
            (ident, colon, expression) => {
                self.add_error(ParseError {
                    kind: ParseErrorKind::InvalidSyntax(format!("Unable to parse arg fields!")),
                    range: Range::default(),
                });
                Some(Arg {
                    name: ident.cloned(),
                    colon: colon.cloned(),
                    value: expression,
                })
            }
        }
    }

    // fn parse_expression(&self) -> Option<Expression> {
    //     self.parse_literal()
    // }

    // fn parse_literal(&self) -> Option<Expression> {
    //     match self.tokens.peek() {
    //         Some(Token::Ident(_)) => Some(Expression::Ident(self.tokens.next().cloned().unwrap())),
    //         _ => None,
    //     }
    // }

    pub(crate) fn expect_operator(&self, operator: Operator) -> Option<&SpannedToken> {
        self.ignore_ws();
        let Some(Token::Operator(o)) = self.tokens.peek() else {
            return None;
        };

        if o == &operator {
            return self.tokens.next();
        }

        None
    }

    pub fn ignore_ws(&self) {
        while let Some(Token::Newline) = self.tokens.peek() {
            self.tokens.next();
        }
    }

    pub(crate) fn expect(&self, token_type: Token) -> Option<&SpannedToken> {
        self.ignore_ws();
        let Some(tok) = self.tokens.peek() else {
            return None;
        };
        if std::mem::discriminant(tok) == std::mem::discriminant(&token_type) {
            return self.tokens.next();
        }

        None
    }
}
