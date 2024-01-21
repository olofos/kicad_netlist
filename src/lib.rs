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
    #[error("Unknown pin type {0}")]
    UnknownPinType(String),
    #[error("Nom error {0}")]
    ParseError(#[from] nom::error::Error<String>),
}

#[derive(Debug, Clone)]
pub struct NetList<'a> {
    pub components: Vec<Component<'a>>,
    pub parts: Vec<Part<'a>>,
    pub nets: Vec<Net<'a>>,
}

#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub reference: &'a str,
    pub value: &'a str,
    pub lib: &'a str,
    pub part: &'a str,
    pub properties: Vec<(&'a str, &'a str)>,
    pub footprint: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinType {
    Input,
    Output,
    Bidirectional,
    TriState,
    Passive,
    Free,
    PowerInput,
    PowerOutput,
    OpenCollector,
    OpenEmitter,
    Unconnected,
}

#[derive(Debug, Clone)]
pub struct Pin<'a> {
    pub num: &'a str,
    pub name: &'a str,
    pub typ: PinType,
}

#[derive(Debug, Clone)]
pub struct Part<'a> {
    pub lib: &'a str,
    pub part: &'a str,
    pub description: &'a str,
    pub pins: Vec<Pin<'a>>,
}

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub reference: &'a str,
    pub pin: &'a str,
    pub function: Option<&'a str>,
    pub typ: PinType,
}

#[derive(Debug, Clone)]
pub struct Net<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub nodes: Vec<Node<'a>>,
}

impl TryFrom<&str> for PinType {
    type Error = NetListParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "input" => Ok(Self::Input),
            "output" => Ok(Self::Output),
            "bidirectional" => Ok(Self::Bidirectional),
            "tri_state" => Ok(Self::TriState),
            "passive" => Ok(Self::Passive),
            "free" => Ok(Self::Free),
            "power_in" => Ok(Self::PowerInput),
            "power_out" => Ok(Self::PowerOutput),
            "open_collector" => Ok(Self::OpenCollector),
            "open_emitter" => Ok(Self::OpenEmitter),
            "no_connect" => Ok(Self::Unconnected),
            s => Err(NetListParseError::UnknownPinType(s.to_owned())),
        }
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Component<'a> {
    type Error = NetListParseError;
    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let reference = value.value("ref")?;
        let val = value.value("value")?;
        let footprint = value.value("footprint").ok();

        let properties = value
            .children("property")
            .map(|prop| {
                let name = prop.value("name")?;
                let value = prop.value("value")?;
                Ok((name, value))
            })
            .collect::<Result<_, Self::Error>>()?;

        let (lib, part) = {
            let libsource = value.child("libsource")?;
            (libsource.value("lib")?, libsource.value("part")?)
        };

        Ok(Self {
            reference,
            value: val,
            lib,
            part,
            properties,
            footprint,
        })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Pin<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let num = value.value("num")?;
        let name = value.value("name")?;
        let typ = value.value("type")?.try_into()?;

        Ok(Pin { num, name, typ })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Part<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let lib = value.value("lib")?;
        let part = value.value("part")?;
        let description = value.value("description")?;
        let pins = value
            .child("pins")?
            .children("pin")
            .map(|pin| pin.try_into())
            .collect::<Result<_, _>>()?;
        Ok(Part {
            lib,
            part,
            description,
            pins,
        })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Node<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let reference = value.value("ref")?;
        let pin = value.value("pin")?;
        let function = value.value("pinfunction").ok();
        let typ = value.value("pintype")?.try_into()?;

        Ok(Node {
            reference,
            pin,
            function,
            typ,
        })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Net<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let code = value.value("code")?;
        let name = value.value("name")?;
        let nodes = value
            .children("node")
            .map(|node| node.try_into())
            .collect::<Result<Vec<_>, Self::Error>>()?;
        Ok(Net { code, name, nodes })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for NetList<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let components: Vec<Component<'a>> = value
            .child("components")?
            .children("comp")
            .map(|comp| comp.try_into())
            .collect::<Result<_, _>>()?;

        let parts = value
            .child("libparts")?
            .children("libpart")
            .map(|part| part.try_into())
            .collect::<Result<_, _>>()?;

        let nets = value
            .child("nets")?
            .children("net")
            .map(|net| net.try_into())
            .collect::<Result<_, _>>()?;

        Ok(NetList {
            components,
            parts,
            nets,
        })
    }
}

impl<'a> TryFrom<SExpr<'a>> for NetList<'a> {
    type Error = NetListParseError;

    fn try_from(value: SExpr<'a>) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl<'a> TryFrom<&'a str> for NetList<'a> {
    type Error = NetListParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let root = root_sexpr(value)?;
        root.try_into()
    }
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

    #[test]
    fn can_parse_comp() {
        let i = &test_data!("kvt.net");
        let root = root_sexpr(i).unwrap();

        let comps: Vec<Component> = root
            .child("components")
            .unwrap()
            .children("comp")
            .map(|expr| expr.try_into().unwrap())
            .collect();

        assert_eq!(comps.len(), 4);
    }

    #[test]
    fn can_parse_part() {
        let i = &test_data!("kvt.net");
        let root = root_sexpr(i).unwrap();

        let parts: Vec<Part> = root
            .child("libparts")
            .unwrap()
            .children("libpart")
            .map(|expr| expr.try_into().unwrap())
            .collect();

        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn can_parse_net() {
        let i = &test_data!("kvt.net");
        let root = root_sexpr(i).unwrap();

        let nets: Vec<Net> = root
            .child("nets")
            .unwrap()
            .children("net")
            .map(|expr| expr.try_into().unwrap())
            .collect();

        assert_eq!(nets.len(), 7);
    }

    #[test]
    fn can_parse_netlist() {
        let i = &test_data!("kvt.net");
        let root = root_sexpr(i).unwrap();
        let netlist: NetList = root.try_into().unwrap();

        assert_eq!(netlist.components.len(), 4);
        assert_eq!(netlist.parts.len(), 3);
        assert_eq!(netlist.nets.len(), 7);
    }
}
