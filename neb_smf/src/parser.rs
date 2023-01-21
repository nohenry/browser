use std::sync::{RwLock, RwLockReadGuard};

use crate::{
    ast::{Arg, ElementArgs, Expression, PunctuationList, Statement},
    error::{ParseError, ParseErrorKind},
    token::{Operator, Range, SpannedToken, Token, TokenStream},
};

pub struct Parser {
    tokens: TokenStream,
    errors: RwLock<Vec<ParseError>>,
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
        while let Some(stmt) = self.parse_statement() {
            statements.push(stmt);

            if let Some(Token::Newline) = self.tokens.peek() {
                self.tokens.next();
            }
            self.ignore_ws();
        }

        Some(statements)
    }

    pub fn parse_statement(&self) -> Option<Statement> {
        let tok = match self.tokens.peek() {
            Some(Token::Ident(_)) => self.tokens.next().unwrap(),
            _ => return None,
        };

        match self.tokens.peek() {
            Some(Token::Operator(Operator::OpenBrace | Operator::OpenParen)) => {
                return self.parse_element(tok)
            }
            _ => return None,
        }
    }

    pub fn parse_element(&self, ident: &SpannedToken) -> Option<Statement> {
        let args = if let Some(Token::Operator(Operator::OpenParen)) = self.tokens.peek() {
            self.parse_args()
        } else {
            None
        };

        let Some(open_brace) = self.expect_operator(Operator::OpenBrace) else {
            return None;
        };
        let mut statements = Vec::new();
        while let Some(stmt) = self.parse_statement() {
            statements.push(stmt);
            if let Some(Token::Operator(Operator::CloseBrace)) = self.tokens.peek() {
                break;
            }
        }

        let Some(close_brace) = self.tokens.next() else {
            return None;
        };

        let st = if let Token::Ident(i) = ident.tok() {
            i.to_string()
        } else {
            return None;
        };

        Some(Statement::Element {
            name: st,
            arguments: args,
            body: statements,
            body_range: Range {
                start: *open_brace.span(),
                end: *close_brace.span(),
            },
            token: ident.clone(),
        })
    }

    fn parse_args(&self) -> Option<ElementArgs> {
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
            None
        }
    }

    fn parse_arg(&self) -> Option<Arg> {
        let ident = self.expect(Token::Ident("".into()));
        let colon = self.expect_operator(Operator::Colon);
        let expression = self.parse_expression();

        if let (Some(ident), Some(colon), Some(expr)) = (ident, colon, expression) {
            Some(Arg {
                name: ident.clone(),
                colon: colon.clone(),
                value: expr,
            })
        } else {
            self.add_error(ParseError {
                kind: ParseErrorKind::InvalidSyntax(format!("Unable to parse arg fields!")),
                range: Range::default(),
            });
            None
        }
    }

    fn parse_expression(&self) -> Option<Expression> {
        self.parse_literal()
    }

    fn parse_literal(&self) -> Option<Expression> {
        match self.tokens.peek() {
            Some(Token::Ident(_)) => Some(Expression::Ident(self.tokens.next().cloned().unwrap())),
            _ => None,
        }
    }

    fn expect_operator(&self, operator: Operator) -> Option<&SpannedToken> {
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

    fn expect(&self, token_type: Token) -> Option<&SpannedToken> {
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
