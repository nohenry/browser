use crate::token::{Operator, Span, SpannedToken, Token, Unit};

pub struct Lexer {}

impl Lexer {
    pub fn lex(&mut self, input: &str) -> Vec<SpannedToken> {
        let mut start_index = 0;
        let mut end_index = 1;

        let mut line_num = 0;
        let mut position = 0;

        let mut str_index: Option<(usize, usize)> = None;

        let mut tokens: Vec<SpannedToken> = Vec::new();
        while start_index < input.len() && end_index <= input.len() {
            let sub_str = &input[start_index..end_index];
            let next = input.chars().nth(end_index);

            if let Some(token) = self.try_lex(sub_str, next) {
                match token {
                    Token::Whitespace => position += 1,
                    Token::Newline => {
                        let ce = end_index - 1;
                        if &input[start_index..end_index + 1] == "\r\n" {
                            end_index += 1;
                            // continue;
                        }
                        if let Some(indicies) = str_index {
                            let st = &input[indicies.1 + 1..ce];
                            if verify_text(st) {
                                let token = SpannedToken::new(
                                    Token::Text(st.to_string()),
                                    Span {
                                        line_num: tokens[indicies.0 as usize].span().line_num,
                                        position: tokens[indicies.0 as usize].span().position,
                                        length: st.len() as u32,
                                        token_index: tokens.len() as u32,
                                    },
                                );

                                tokens.truncate(indicies.0);

                                tokens.push(token);
                            }
                        }

                        line_num += 1;
                        position = 0;

                        str_index = None
                    }
                    Token::Ident(_) => {
                        let token = SpannedToken::new(
                            token,
                            Span {
                                line_num,
                                position,
                                length: (end_index - start_index) as u32,
                                token_index: tokens.len() as u32,
                            },
                        );

                        // if str_index.is_none()
                        //     && tokens
                        //         .last()
                        //         .map(|c| c.0.line_num < token.0.line_num)
                        //         .unwrap_or(false)
                        // {
                        //     str_index = Some((tokens.len(), start_index));
                        // }

                        tokens.push(token);
                        position += (end_index - start_index) as u32;
                    }
                    Token::Operator(Operator::Colon) => {
                        let token = SpannedToken::new(
                            token,
                            Span {
                                line_num,
                                position,
                                length: (end_index - start_index) as u32,
                                token_index: tokens.len() as u32,
                            },
                        );

                        // If the token starts at the beginning of a line
                        if str_index.is_none()
                            && tokens
                                .last()
                                .map(|c| c.0.line_num < token.0.line_num)
                                .unwrap_or(false)
                        {
                            str_index = Some((tokens.len(), start_index));
                        }
                        tokens.push(token);
                        position += (end_index - start_index) as u32;
                    }
                    Token::Integer(i, _) => {
                        let unit = if &input[end_index..end_index + 2] == "px" {
                            end_index += 2;
                            Some(Unit::Pixel)
                        } else {
                            None
                        };

                        let token = SpannedToken::new(
                            Token::Integer(i, unit),
                            Span {
                                line_num,
                                position,
                                length: (end_index - start_index) as u32,
                                token_index: tokens.len() as u32,
                            },
                        );

                        tokens.push(token);
                        position += (end_index - start_index) as u32;
                    }
                    Token::Float(f, _) => {
                        let unit = if &input[end_index..end_index + 2] == "px" {
                            end_index += 2;
                            Some(Unit::Pixel)
                        } else {
                            None
                        };

                        let token = SpannedToken::new(
                            Token::Float(f, unit),
                            Span {
                                line_num,
                                position,
                                length: (end_index - start_index) as u32,
                                token_index: tokens.len() as u32,
                            },
                        );

                        tokens.push(token);
                        position += (end_index - start_index) as u32;
                    }
                    token => {
                        let token = SpannedToken::new(
                            token,
                            Span {
                                line_num,
                                position,
                                length: (end_index - start_index) as u32,
                                token_index: tokens.len() as u32,
                            },
                        );

                        tokens.push(token);
                        position += (end_index - start_index) as u32;
                    }
                }

                start_index = end_index;
                end_index = start_index + 1;
            } else {
                end_index += 1;
            }
        }

        tokens.push(SpannedToken::new(
            Token::Newline,
            Span {
                line_num,
                position,
                length: 1,
                token_index: tokens.len() as u32,
            },
        ));

        tokens
    }

    pub fn try_lex<'a>(&mut self, input: &'a str, next: Option<char>) -> Option<Token> {
        if input.len() == 1 {
            // match single character symbols
            match input.chars().nth(0) {
                Some('[') => return Some(Token::Operator(Operator::OpenSquare)),
                Some(']') => return Some(Token::Operator(Operator::CloseSquare)),
                Some('(') => return Some(Token::Operator(Operator::OpenParen)),
                Some(')') => return Some(Token::Operator(Operator::CloseParen)),
                Some('{') => return Some(Token::Operator(Operator::OpenBrace)),
                Some('}') => return Some(Token::Operator(Operator::CloseBrace)),
                Some(':') => return Some(Token::Operator(Operator::Colon)),
                Some('.') => return Some(Token::Operator(Operator::Dot)),
                Some(',') => return Some(Token::Operator(Operator::Comma)),
                Some('\r' | '\n') => return Some(Token::Newline),
                Some(c) if c.is_whitespace() => return Some(Token::Whitespace),
                _ => (),
            }
        }

        let del = match next.map(|c| !(c.is_numeric() || c == '.')) {
            None => true,
            Some(t) => t,
        };

        let cnt = input
            .chars()
            .fold(0u8, |acc, c| if c == '.' { 1 + acc } else { acc });
        if input
            .chars()
            .find(|c| !(c.is_numeric() || *c == '.'))
            .is_none()
            && cnt <= 1
            && del
        {
            if cnt == 1 {
                let val = input.parse().unwrap_or(0.0f64);
                return Some(Token::Float(val, None));
            } else {
                let val = input.parse().unwrap_or(0u64);
                return Some(Token::Integer(val, None));
            }
        }

        // If the next character is a delimeter
        let del = match next.map(|c| !(c.is_alphabetic() || c == '_')) {
            None => true,
            Some(t) => t,
        };

        // match identifiers
        if input
            .chars()
            .find(|c| !(c.is_alphabetic() || *c == '_'))
            .is_none()
            && del
        {
            return Some(Token::Ident(input.to_string()));
        }

        if let Some('\n' | '\r') = next {
            return Some(Token::Text(input.to_string()));
        }

        None
    }
}

fn verify_text(st: &str) -> bool {
    let val = st.chars().find(|c| {
        !(match c {
            ' ' | ',' | '\'' | '"' | '!' | '@' | '#' | '$' | '%' | '^' | '&' | '*' | '(' | ')'
            | '[' | ']' | '?' | '/' | ';' | ':' | '\\' | '.' | '<' | '>' | '-' | '_' | '+'
            | '=' => true,
            c => c.is_alphanumeric(),
        })
    });

    println!("Val: {:?}", val);

    val.is_none()
}

// fn match_str_no_case(a: &str, b: &str) -> bool {
//     if a.len() != b.len() {
//         return false;
//     }

//     a.chars()
//         .zip(b.chars())
//         .find(|(a, b)| a.to_ascii_lowercase() != b.to_ascii_lowercase())
//         .is_none()
// }
