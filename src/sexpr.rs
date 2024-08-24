use std::fmt::Display;

use crate::error::ParseError;

mod lexer;
mod parser;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SExpr<'a> {
    SExpr(&'a str, Box<[SExpr<'a>]>),
    String(&'a str),
}

impl<'a> Display for SExpr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SExpr::SExpr(label, children) => {
                write!(f, "({}", label)?;
                for child in children {
                    write!(f, " {}", child)?;
                }
                write!(f, ")")
            }
            SExpr::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

impl<'a> SExpr<'a> {
    pub fn value(&self, label: &str) -> Result<&'a str, ParseError> {
        let child = self.child(label)?;
        if let SExpr::SExpr(_, children) = child {
            if !children.is_empty() {
                match children[0] {
                    SExpr::String(s) => return Ok(s),
                    SExpr::SExpr(_, _) => {}
                }
            };
        }
        Err(ParseError::MissingValue())
    }

    pub fn children<'b, 'c>(&'b self, label: &'c str) -> LabeledChildIterator<'a, 'b, 'c> {
        let iter = match self {
            SExpr::String(_) => None,
            SExpr::SExpr(_, children) => Some(children.iter()),
        };
        LabeledChildIterator { iter, label }
    }

    pub fn child<'b>(&self, label: &'b str) -> Result<&SExpr<'a>, ParseError> {
        let mut iter = self.children(label);
        iter.next()
            .ok_or(ParseError::MissingChild(label.to_owned()))
    }
}

#[derive(Debug)]
pub struct LabeledChildIterator<'a, 'b, 'c> {
    iter: Option<std::slice::Iter<'b, SExpr<'a>>>,
    label: &'c str,
}

impl<'a, 'b, 'c> Iterator for LabeledChildIterator<'a, 'b, 'c> {
    type Item = &'b SExpr<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let iter = self.iter.as_mut()?;
        loop {
            let item = iter.next();
            match &item {
                None => return None,
                Some(SExpr::String(_)) => continue,
                Some(SExpr::SExpr(label, _)) => {
                    if *label == self.label {
                        return item;
                    }
                }
            }
        }
    }
}

impl<'a> TryFrom<&'a String> for SExpr<'a> {
    type Error = ParseError;

    fn try_from(input: &'a String) -> Result<Self, Self::Error> {
        SExpr::try_from(input.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_data {
        ($fname:expr) => {
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/test/",
                $fname
            ))
            .unwrap()
        };
    }
    #[test]
    fn sexpr_can_parse_full_file() {
        let i = &test_data!("kvt.net");
        let _ = SExpr::try_from(i).unwrap();
    }
}
