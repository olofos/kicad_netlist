use logos::{Logos, SpannedIter};

pub(super) struct Token {
    pub(super) kind: TokenKind,
    pub(super) span: logos::Span,
}

pub(super) struct TokenIter<'a> {
    iter: SpannedIter<'a, LogosTokenKind>,
}

impl<'a> TokenIter<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            iter: LogosTokenKind::lexer(input).spanned(),
        }
    }
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some((Ok(LogosTokenKind::QuotedString), span)) => {
                let span = (span.start + 1)..(span.end - 1);
                Some(Token {
                    kind: TokenKind::String,
                    span,
                })
            }
            Some((Ok(kind), span)) => {
                let (kind, span) = match kind {
                    LogosTokenKind::LParen => (TokenKind::LParen, span),
                    LogosTokenKind::RParen => (TokenKind::RParen, span),
                    LogosTokenKind::QuotedString => {
                        (TokenKind::String, (span.start + 1)..(span.end - 1))
                    }
                    LogosTokenKind::String => (TokenKind::String, span),
                    LogosTokenKind::WS => unreachable!(),
                };
                Some(Token { kind, span })
            }
            Some((Err(_), span)) => Some(Token {
                kind: TokenKind::Error,
                span,
            }),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TokenKind {
    LParen,
    RParen,
    String,
    Error,
}

#[derive(Logos, Clone, Copy, Debug, PartialEq, Eq)]
enum LogosTokenKind {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    QuotedString,
    #[regex(r#"([^"() \t\r\f\n])*"#)]
    String,
    #[regex(r"[ \t\r\f\n]+", logos::skip)]
    WS,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let input = "(a \"b\" \"\" \n)";
        let mut it = TokenIter {
            iter: LogosTokenKind::lexer(input).spanned(),
        };
        let expected = vec![
            (TokenKind::LParen, "("),
            (TokenKind::String, "a"),
            (TokenKind::String, "b"),
            (TokenKind::String, ""),
            (TokenKind::RParen, ")"),
        ];

        let mut result = vec![];

        while let Some(token) = it.next() {
            result.push((token.kind, &input[token.span.clone()]));
        }

        assert_eq!(result, expected);
    }
}
