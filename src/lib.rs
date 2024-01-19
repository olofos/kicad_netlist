use nom::{
    branch::alt,
    bytes::complete::{escaped, tag},
    character::complete::{alpha1, multispace0, none_of, one_of},
    combinator::{map, recognize, value},
    multi::{many0, many1},
    sequence::{delimited, terminated, tuple},
    IResult,
};

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

fn sexpr<'a, O, E, F>(
    label: &'a str,
    arg_parser: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str>,
    F: nom::Parser<&'a str, O, E>,
{
    delimited(
        tuple((tag("("), tag(label), multispace0)),
        arg_parser,
        tuple((tag(")"), multispace0)),
    )
}

fn string(i: &str) -> IResult<&str, &str> {
    alt((
        value("", tag("\"\"")),
        delimited(
            tag("\""),
            escaped(none_of(r#"\""#), '\\', one_of(r#""nfrtb\"#)),
            tag("\""),
        ),
    ))(i)
}

fn field(i: &str) -> IResult<&str, (&str, &str)> {
    sexpr("field", tuple((sexpr("name", string), string)))(i)
}

fn property(i: &str) -> IResult<&str, (&str, &str)> {
    sexpr(
        "property",
        tuple((sexpr("name", string), sexpr("value", string))),
    )(i)
}

fn libsource(i: &str) -> IResult<&str, (&str, &str)> {
    let (i, (lib, part, _)) = sexpr(
        "libsource",
        tuple((
            sexpr("lib", string),
            sexpr("part", string),
            opt(sexpr("description", string)),
        )),
    )(i)?;

    Ok((i, (lib, part)))
}

fn comp(i: &str) -> IResult<&str, Component> {
    let (i, (reference, value, footprint, _, _, (lib, part), properties, _, _)) = sexpr(
        "comp",
        tuple((
            sexpr("ref", string),
            sexpr("value", string),
            opt(sexpr("footprint", string)),
            opt(sexpr("datasheet", string)),
            opt(sexpr("fields", many0(field))),
            libsource,
            many0(property),
            sexpr(
                "sheetpath",
                tuple((sexpr("names", string), sexpr("tstamps", string))),
            ),
            sexpr("tstamps", string),
        )),
    )(i)?;

    Ok((
        i,
        Component {
            reference,
            value,
            lib,
            part,
            properties,
            footprint,
        },
    ))
}

#[derive(Debug)]
enum SExpr<'a> {
    SExpr(&'a str, Vec<SExpr<'a>>),
    String(&'a str),
}

impl<'a> SExpr<'a> {
    fn is_string(&self) -> bool {
        matches!(self, SExpr::String(_))
    }

    fn label(&self) -> Option<&str> {
        match self {
            SExpr::String(_) => None,
            SExpr::SExpr(label, _) => Some(label),
        }
    }

    fn value(&self) -> Option<&str> {
        match self {
            SExpr::String(s) => Some(s),
            SExpr::SExpr(_, children) => {
                if children.is_empty() {
                    None
                } else {
                    children[0].value()
                }
            }
        }
    }
}

#[derive(Debug)]
struct LabeledChildIterator<'a, 'b> {
    iter: Option<std::slice::Iter<'a, SExpr<'a>>>,
    label: &'b str,
}

impl<'a, 'b> Iterator for LabeledChildIterator<'a, 'b> {
    type Item = &'a SExpr<'a>;

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

impl<'a> SExpr<'a> {
    fn children<'b>(&'a self, label: &'b str) -> LabeledChildIterator<'a, 'b> {
        let iter = match self {
            SExpr::String(_) => None,
            SExpr::SExpr(_, children) => Some(children.iter()),
        };
        LabeledChildIterator { iter, label }
    }

    fn child<'b>(&'a self, label: &'b str) -> Option<&'a SExpr<'a>> {
        let mut iter = self.children(label);
        iter.next()
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sexpr_works() {
        let result: IResult<_, _, ()> = sexpr("test", tag("test"))("(test test)");
        assert_eq!(result.unwrap(), ("", "test"));
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
        assert!(children[0].is_string());
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
    fn sheet_works() {
        let input = r#"(sheet (number "1") (name "/") (tstamps "/")
        (title_block
          (title "Title")
          (company "Acme Inc")
          (rev "A")
          (date "2024-01-16")
          (source "netlist-test.kicad_sch")
          (comment (number "1") (value "comment"))
          (comment (number "2") (value ""))
          (comment (number "9") (value ""))))"#;
        let (i, _) = sheet(input).unwrap();
        assert_eq!(i, "");
    }

    #[test]
    fn netlist_works() {
        let input = r#"(export (version "E")
        (design
          (source "C:\\test\\netlist-test.kicad_sch")
          (date "2024-01-17 00:01:48")
          (tool "Eeschema 7.0.7")
          (sheet (number "1") (name "/") (tstamps "/")
            (title_block
              (title "Title")
              (company "Acme Inc")
              (rev "A")
              (date "2024-01-16")
              (source "netlist-test.kicad_sch")
              (comment (number "1") (value "comment"))
              (comment (number "2") (value ""))))
          (sheet (number "2") (name "/Sub Sheet/") (tstamps "/520c242b-a023-4915-8159-1d083b2b529d/")
            (title_block
              (title)
              (company)
              (rev "A")
              (date)
              (source "sub.kicad_sch")))
          (sheet (number "3") (name "/Sub Sheet1/") (tstamps "/9147f5ed-c0cc-4b23-8044-fb0e92e595ca/")
            (title_block
              (title)
              (company)
              (rev "A")
              (date)
              (source "sub.kicad_sch"))))
        (components
          (comp (ref "U1")
            (value "74AHC1G00")
            (datasheet "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf")
            (libsource (lib "74xGxx") (part "74AHC1G00") (description "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "Sheetname") (value "Sub Sheet"))
            (property (name "Sheetfile") (value "sub.kicad_sch"))
            (property (name "ki_description") (value "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "ki_keywords") (value "Single Gate NAND LVC CMOS"))
            (sheetpath (names "/Sub Sheet/") (tstamps "/520c242b-a023-4915-8159-1d083b2b529d/"))
            (tstamps "2b032933-729e-48b1-a1f3-8e684e2740b3"))
          (comp (ref "U2")
            (value "74AHC1G00")
            (datasheet "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf")
            (libsource (lib "74xGxx") (part "74AHC1G00") (description "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "Sheetname") (value "Sub Sheet1"))
            (property (name "Sheetfile") (value "sub.kicad_sch"))
            (property (name "ki_description") (value "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "ki_keywords") (value "Single Gate NAND LVC CMOS"))
            (sheetpath (names "/Sub Sheet1/") (tstamps "/9147f5ed-c0cc-4b23-8044-fb0e92e595ca/"))
            (tstamps "2b032933-729e-48b1-a1f3-8e684e2740b3")))
        (libparts
          (libpart (lib "74xGxx") (part "74AHC1G00")
            (description "Single NAND Gate, Low-Voltage CMOS")
            (docs "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf")
            (footprints
              (fp "SOT*")
              (fp "SG-*"))
            (fields
              (field (name "Reference") "U")
              (field (name "Value") "74AHC1G00")
              (field (name "Datasheet") "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf"))
            (pins
              (pin (num "1") (name "") (type "input"))
              (pin (num "2") (name "") (type "input"))
              (pin (num "3") (name "GND") (type "power_in"))
              (pin (num "4") (name "") (type "output"))
              (pin (num "5") (name "VCC") (type "power_in")))))
        (libraries
          (library (logical "74xGxx")
            (uri "C:\\Program Files\\KiCad\\7.0\\share\\kicad\\symbols\\/74xGxx.kicad_sym")))
        (nets
          (net (code "1") (name "/Sub Sheet/In")
            (node (ref "U1") (pin "1") (pintype "input"))
            (node (ref "U2") (pin "4") (pintype "output")))
          (net (code "2") (name "/Sub Sheet/Out")
            (node (ref "U1") (pin "4") (pintype "output"))
            (node (ref "U2") (pin "1") (pintype "input")))
          (net (code "3") (name "Net-(U1-GND)")
            (node (ref "U1") (pin "2") (pintype "input"))
            (node (ref "U1") (pin "3") (pinfunction "GND") (pintype "power_in")))
          (net (code "4") (name "Net-(U2-GND)")
            (node (ref "U2") (pin "2") (pintype "input"))
            (node (ref "U2") (pin "3") (pinfunction "GND") (pintype "power_in")))
          (net (code "5") (name "VCC")
            (node (ref "U1") (pin "5") (pinfunction "VCC") (pintype "power_in"))
            (node (ref "U2") (pin "5") (pinfunction "VCC") (pintype "power_in")))))"#;

        let (i, netlist) = netlist(input).unwrap();
        assert_eq!(i, "");
        assert_eq!(netlist.components.len(), 2);
        assert_eq!(netlist.parts.len(), 1);
        assert_eq!(netlist.nets.len(), 5);
    }

    #[test]
    fn netlist_try_from_works() {
        let input = r#"(export (version "E")
        (design
          (source "C:\\test\\netlist-test.kicad_sch")
          (date "2024-01-17 00:01:48")
          (tool "Eeschema 7.0.7")
          (sheet (number "1") (name "/") (tstamps "/")
            (title_block
              (title "Title")
              (company "Acme Inc")
              (rev "A")
              (date "2024-01-16")
              (source "netlist-test.kicad_sch")
              (comment (number "1") (value "comment"))
              (comment (number "2") (value ""))))
          (sheet (number "2") (name "/Sub Sheet/") (tstamps "/520c242b-a023-4915-8159-1d083b2b529d/")
            (title_block
              (title)
              (company)
              (rev "A")
              (date)
              (source "sub.kicad_sch")))
          (sheet (number "3") (name "/Sub Sheet1/") (tstamps "/9147f5ed-c0cc-4b23-8044-fb0e92e595ca/")
            (title_block
              (title)
              (company)
              (rev "A")
              (date)
              (source "sub.kicad_sch"))))
        (components
          (comp (ref "U1")
            (value "74AHC1G00")
            (datasheet "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf")
            (libsource (lib "74xGxx") (part "74AHC1G00") (description "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "Sheetname") (value "Sub Sheet"))
            (property (name "Sheetfile") (value "sub.kicad_sch"))
            (property (name "ki_description") (value "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "ki_keywords") (value "Single Gate NAND LVC CMOS"))
            (sheetpath (names "/Sub Sheet/") (tstamps "/520c242b-a023-4915-8159-1d083b2b529d/"))
            (tstamps "2b032933-729e-48b1-a1f3-8e684e2740b3"))
          (comp (ref "U2")
            (value "74AHC1G00")
            (datasheet "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf")
            (libsource (lib "74xGxx") (part "74AHC1G00") (description "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "Sheetname") (value "Sub Sheet1"))
            (property (name "Sheetfile") (value "sub.kicad_sch"))
            (property (name "ki_description") (value "Single NAND Gate, Low-Voltage CMOS"))
            (property (name "ki_keywords") (value "Single Gate NAND LVC CMOS"))
            (sheetpath (names "/Sub Sheet1/") (tstamps "/9147f5ed-c0cc-4b23-8044-fb0e92e595ca/"))
            (tstamps "2b032933-729e-48b1-a1f3-8e684e2740b3")))
        (libparts
          (libpart (lib "74xGxx") (part "74AHC1G00")
            (description "Single NAND Gate, Low-Voltage CMOS")
            (docs "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf")
            (footprints
              (fp "SOT*")
              (fp "SG-*"))
            (fields
              (field (name "Reference") "U")
              (field (name "Value") "74AHC1G00")
              (field (name "Datasheet") "http://www.ti.com/lit/sg/scyt129e/scyt129e.pdf"))
            (pins
              (pin (num "1") (name "") (type "input"))
              (pin (num "2") (name "") (type "input"))
              (pin (num "3") (name "GND") (type "power_in"))
              (pin (num "4") (name "") (type "output"))
              (pin (num "5") (name "VCC") (type "power_in")))))
        (libraries
          (library (logical "74xGxx")
            (uri "C:\\Program Files\\KiCad\\7.0\\share\\kicad\\symbols\\/74xGxx.kicad_sym")))
        (nets
          (net (code "1") (name "/Sub Sheet/In")
            (node (ref "U1") (pin "1") (pintype "input"))
            (node (ref "U2") (pin "4") (pintype "output")))
          (net (code "2") (name "/Sub Sheet/Out")
            (node (ref "U1") (pin "4") (pintype "output"))
            (node (ref "U2") (pin "1") (pintype "input")))
          (net (code "3") (name "Net-(U1-GND)")
            (node (ref "U1") (pin "2") (pintype "input"))
            (node (ref "U1") (pin "3") (pinfunction "GND") (pintype "power_in")))
          (net (code "4") (name "Net-(U2-GND)")
            (node (ref "U2") (pin "2") (pintype "input"))
            (node (ref "U2") (pin "3") (pinfunction "GND") (pintype "power_in")))
          (net (code "5") (name "VCC")
            (node (ref "U1") (pin "5") (pinfunction "VCC") (pintype "power_in"))
            (node (ref "U2") (pin "5") (pinfunction "VCC") (pintype "power_in")))))"#;

        let netlist: NetList = input.try_into().unwrap();
        assert_eq!(netlist.components.len(), 2);
        assert_eq!(netlist.parts.len(), 1);
        assert_eq!(netlist.nets.len(), 5);
    }
}
