use nom::{
    branch::alt,
    bytes::complete::{escaped, tag},
    character::complete::{alpha1, multispace0, none_of, one_of},
    combinator::{map, recognize, value},
    multi::{many0, many1},
    sequence::{delimited, terminated, tuple},
    Finish, IResult,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetListParseError {
    #[error("SExpr {0} not found")]
    MissingChild(String),
    #[error("Value not found")]
    MissingValue(),
    #[error("Nom error {0}")]
    ParseError(#[from] nom::error::Error<String>),
}

pub struct NetList<'a> {
    pub components: Vec<Component<'a>>,
    pub parts: Vec<Part<'a>>,
    pub nets: Vec<Net<'a>>,
}

pub struct Component<'a> {
    pub reference: &'a str,
    pub value: &'a str,
    pub lib: &'a str,
    pub part: &'a str,
    pub properties: Vec<(&'a str, &'a str)>,
    pub footprint: Option<&'a str>,
}

pub struct Pin<'a> {
    pub num: &'a str,
    pub name: &'a str,
    pub typ: &'a str,
}

pub struct Part<'a> {
    pub lib: &'a str,
    pub part: &'a str,
    pub description: &'a str,
    pub pins: Vec<Pin<'a>>,
}

pub struct Node<'a> {
    pub reference: &'a str,
    pub pin: &'a str,
    pub function: Option<&'a str>,
    pub typ: &'a str,
}

pub struct Net<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub nodes: Vec<Node<'a>>,
}

#[derive(Debug)]
enum SExpr<'a> {
    SExpr(&'a str, Vec<SExpr<'a>>),
    String(&'a str),
}

impl<'a> SExpr<'a> {
    fn value(&self, label: &str) -> Result<&'a str, NetListParseError> {
        let child = self.child(label)?;
        if let SExpr::SExpr(_, children) = child {
            if !children.is_empty() {
                match children[0] {
                    SExpr::String(s) => return Ok(s),
                    SExpr::SExpr(_, _) => {}
                }
            };
        }
        Err(NetListParseError::MissingValue())
    }

    fn children<'b, 'c>(&'b self, label: &'c str) -> LabeledChildIterator<'a, 'b, 'c> {
        let iter = match self {
            SExpr::String(_) => None,
            SExpr::SExpr(_, children) => Some(children.iter()),
        };
        LabeledChildIterator { iter, label }
    }

    fn child<'b>(&self, label: &'b str) -> Result<&SExpr<'a>, NetListParseError> {
        let mut iter = self.children(label);
        iter.next()
            .ok_or(NetListParseError::MissingChild(label.to_owned()))
    }
}

#[derive(Debug)]
struct LabeledChildIterator<'a, 'b, 'c> {
    iter: Option<std::slice::Iter<'b, SExpr<'a>>>,
    label: &'c str,
}

impl<'a, 'b, 'c> Iterator for LabeledChildIterator<'a, 'b, 'c> {
    type Item = &'b SExpr<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(ref mut iter) = self.iter else {
            return None;
        };
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

fn string(i: &str) -> IResult<&str, SExpr> {
    map(
        terminated(
            alt((
                value("", tag("\"\"")),
                delimited(
                    tag("\""),
                    escaped(none_of(r#"\""#), '\\', one_of(r#""nfrtb\"#)),
                    tag("\""),
                ),
            )),
            multispace0,
        ),
        SExpr::String,
    )(i)
}

fn label(i: &str) -> IResult<&str, &str> {
    terminated(recognize(many1(alt((alpha1, tag("_"))))), multispace0)(i)
}

fn sexpr(i: &str) -> IResult<&str, SExpr> {
    map(
        delimited(
            tag("("),
            tuple((label, many0(alt((string, sexpr))))),
            tuple((tag(")"), multispace0)),
        ),
        |(label, chilren)| SExpr::SExpr(label, chilren),
    )(i)
}

fn root_sexpr(i: &str) -> Result<SExpr, NetListParseError> {
    let (_, root) = (sexpr)(i).map_err(|e| e.to_owned()).finish()?;
    Ok(root)
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
    fn sexpr_works() {
        let i = r#"(a "b")"#;
        let (i, r) = sexpr(i).unwrap();
        assert_eq!(i, "");
        let SExpr::SExpr(label, children) = r else {
            panic!("")
        };
        assert_eq!(label, "a");
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn sexpr_children_by_name_works() {
        let i = r#"(a (b "1") (c "2") (b "3"))"#;
        let (_, root) = sexpr(i).unwrap();

        let mut iter = root.children("b");
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn sexpr_can_parse_full_file() {
        let i = &test_data!("kvt.net");
        let _ = sexpr(i).unwrap();
    }
}
