use std::iter::Peekable;

use crate::error::ParseError;

use super::{
    lexer::{Token, TokenIter, TokenKind},
    SExpr,
};

pub(super) struct Parser<'a> {
    input: &'a str,
    iter: Peekable<TokenIter<'a>>,
}

type Span = logos::Span;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParsedSExpr {
    SExpr(Span, Vec<ParsedSExpr>),
    String(Span),
}

impl ParsedSExpr {
    fn into_sexpr(self, input: &str) -> SExpr {
        match self {
            ParsedSExpr::SExpr(label_span, children) => {
                let label = &input[label_span];
                let children: Box<[SExpr]> =
                    children.into_iter().map(|c| c.into_sexpr(input)).collect();
                SExpr::SExpr(label, children)
            }
            ParsedSExpr::String(span) => SExpr::String(&input[span]),
        }
    }
}

impl<'a> Parser<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            input,
            iter: TokenIter::new(input).peekable(),
        }
    }

    fn get(&mut self) -> Result<Token, ParseError> {
        let Some(tok) = self.iter.next() else {
            let end = self.input.len();
            return Err(ParseError::UnexpectedEof {
                at: end..end,
                // source_code: None,
            });
        };
        Ok(tok)
    }

    fn peek(&mut self) -> Option<TokenKind> {
        self.iter.peek().map(|tok| tok.kind)
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        let tok = self.get()?;
        if tok.kind == kind {
            Ok(tok)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", kind),
                found: format!("{:?}", tok.kind),
                at: tok.span.clone(),
            })
        }
    }

    fn skip(&mut self) {
        self.get()
            .expect("skip should not be called after EOF is found");
    }

    fn parse_sexpr(&mut self) -> Result<ParsedSExpr, ParseError> {
        self.expect(TokenKind::LParen)?;
        let label = self.expect(TokenKind::String)?;

        let mut children = Vec::new();
        loop {
            match self.peek() {
                Some(TokenKind::RParen) => {
                    self.skip();
                    break Ok(ParsedSExpr::SExpr(label.span.clone(), children));
                }
                Some(TokenKind::LParen) => {
                    children.push(self.parse_sexpr()?);
                }
                Some(TokenKind::String) => {
                    children.push(ParsedSExpr::String(self.get()?.span.clone()));
                }
                Some(TokenKind::Error) => {
                    let tok = self.get()?;
                    break Err(ParseError::UnknownToken {
                        found: format!("{:?}", tok.kind),
                        at: label.span.clone(),
                    });
                }
                None => {
                    break Err(ParseError::UnexpectedEof {
                        at: self.get()?.span.clone(),
                    })
                }
            }
        }
    }
}

impl<'a> TryFrom<&'a str> for SExpr<'a> {
    type Error = ParseError;

    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let mut parser = Parser::new(input);
        let sexpr = parser.parse_sexpr()?;
        let sexpr = sexpr.into_sexpr(input);
        Ok(sexpr)
    }
}

#[cfg(test)]
mod tests {
    use crate::sexpr::SExpr;
    use rstest::*;

    #[rstest]
    #[case("(abc)", "(abc)")]
    #[case("(abc\n)", "(abc)")]
    fn can_parse_sexpr(#[case] input: &str, #[case] expected: &str) {
        let sexpr = SExpr::try_from(input).unwrap();
        assert_eq!(&format!("{sexpr}"), expected);
    }
}
