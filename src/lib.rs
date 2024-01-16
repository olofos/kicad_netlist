use nom::{
    branch::alt,
    bytes::complete::{escaped, tag},
    character::complete::{multispace0, none_of, one_of},
    combinator::{opt, value},
    error::ParseError,
    multi::{many0, many1},
    sequence::{delimited, tuple},
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

fn pin(i: &str) -> IResult<&str, Pin> {
    let (i, (num, name, typ)) = sexpr(
        "pin",
        tuple((
            sexpr("num", string),
            sexpr("name", string),
            sexpr("type", string),
        )),
    )(i)?;
    Ok((i, Pin { num, name, typ }))
}

fn libpart(i: &str) -> IResult<&str, Part> {
    let (i, (lib, part, description, _, _, _, pins)) = sexpr(
        "libpart",
        tuple((
            sexpr("lib", string),
            sexpr("part", string),
            sexpr("description", string),
            sexpr("docs", string),
            opt(sexpr("footprints", many0(sexpr("fp", string)))),
            opt(sexpr("fields", many0(field))),
            sexpr("pins", many0(pin)),
        )),
    )(i)?;
    Ok((
        i,
        Part {
            lib,
            part,
            description,
            pins,
        },
    ))
}

fn library(i: &str) -> IResult<&str, (&str, &str)> {
    sexpr(
        "library",
        tuple((sexpr("logical", string), sexpr("uri", string))),
    )(i)
}

fn sheet(i: &str) -> IResult<&str, ()> {
    let (i, _) = sexpr(
        "sheet",
        tuple((
            sexpr("number", string),
            sexpr("name", string),
            sexpr("tstamps", string),
            sexpr(
                "title_block",
                tuple((
                    sexpr("title", opt(string)),
                    sexpr("company", opt(string)),
                    sexpr("rev", opt(string)),
                    sexpr("date", opt(string)),
                    sexpr("source", opt(string)),
                    many0(sexpr(
                        "comment",
                        tuple((sexpr("number", string), sexpr("value", string))),
                    )),
                )),
            ),
        )),
    )(i)?;
    Ok((i, ()))
}

fn node(i: &str) -> IResult<&str, Node> {
    let (i, (reference, pin, function, typ)) = sexpr(
        "node",
        tuple((
            sexpr("ref", string),
            sexpr("pin", string),
            opt(sexpr("pinfunction", string)),
            sexpr("pintype", string),
        )),
    )(i)?;
    Ok((
        i,
        Node {
            reference,
            pin,
            function,
            typ,
        },
    ))
}

fn net(i: &str) -> IResult<&str, Net> {
    let (i, (code, name, nodes)) = sexpr(
        "net",
        tuple((sexpr("code", string), sexpr("name", string), many0(node))),
    )(i)?;

    Ok((i, Net { code, name, nodes }))
}

fn netlist(i: &str) -> IResult<&str, NetList> {
    let (i, (_, _, components, parts, _, nets)) = sexpr(
        "export",
        tuple((
            sexpr("version", tag(r#""E""#)),
            sexpr(
                "design",
                tuple((
                    sexpr("source", string),
                    sexpr("date", string),
                    sexpr("tool", string),
                    many1(sheet),
                )),
            ),
            sexpr("components", many0(comp)),
            sexpr("libparts", many0(libpart)),
            sexpr("libraries", many0(library)),
            sexpr("nets", many0(net)),
        )),
    )(i)?;
    Ok((
        i,
        NetList {
            components,
            parts,
            nets,
        },
    ))
}

impl<'a> TryFrom<&'a str> for NetList<'a> {
    type Error = ();

    fn try_from(i: &'a str) -> Result<Self, Self::Error> {
        match netlist(i) {
            Ok((i, netlist)) => {
                if i == "" {
                    return Ok(netlist);
                } else {
                    return Err(());
                }
            }
            Err(_) => return Err(()),
        }
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
    fn string_works() {
        assert_eq!(string(r#""abc""#).unwrap(), ("", "abc"));
    }

    #[test]
    fn string_can_parse_empty_string() {
        assert_eq!(string(r#""""#).unwrap(), ("", ""));
    }

    #[test]
    fn string_escaping_works() {
        assert_eq!(string(r#""\\\"""#).unwrap(), ("", r#"\\\""#));
    }

    #[test]
    fn comp_works() {
        let input = r#"(comp (ref "J1")
        (value "Conn_01x06_Pin")
        (footprint "Connector_PinHeader_2.54mm:PinHeader_1x06_P2.54mm_Vertical")
        (fields
          (field (name "test1") "test 1")
          (field (name "test2") "test \"2\""))
        (libsource (lib "Connector") (part "Conn_01x06_Pin") (description "Generic connector, single row, 01x06, script generated"))
        (property (name "Sheetname") (value ""))
        (property (name "Sheetfile") (value "kvt.kicad_sch"))
        (property (name "ki_description") (value "Generic connector, single row, 01x06, script generated"))
        (property (name "ki_keywords") (value "connector"))
        (sheetpath (names "/") (tstamps "/"))
        (tstamps "73417a21-9c42-4702-9832-ec63427d336d"))"#;
        let (i, comp) = comp(input).unwrap();

        assert_eq!(i, "");
        assert_eq!(comp.reference, "J1");
        assert_eq!(comp.value, "Conn_01x06_Pin");
        assert_eq!(comp.lib, "Connector");
        assert_eq!(comp.part, "Conn_01x06_Pin");
        assert_eq!(comp.properties.len(), 4);
    }

    #[test]
    fn libpart_works() {
        let input = r#"(libpart (lib "74xGxx") (part "74LVC1G00")
        (description "Single NAND Gate, Low-Voltage CMOS")
        (docs "https://www.ti.com/lit/ds/symlink/sn74lvc1g00.pdf")
        (footprints
          (fp "SOT?23*")
          (fp "Texas?R-PDSO-G5?DCK*")
          (fp "Texas?R-PDSO-N5?DRL*")
          (fp "Texas?X2SON*0.8x0.8mm*P0.48mm*"))
        (fields
          (field (name "Reference") "U1")
          (field (name "Value") "74LVC1G00")
          (field (name "Footprint") "Package_TO_SOT_SMD:SOT-23-5_HandSoldering")
          (field (name "Datasheet") "https://www.ti.com/lit/ds/symlink/sn74lvc1g00.pdf"))
        (pins
          (pin (num "1") (name "A") (type "input"))
          (pin (num "2") (name "B") (type "input"))
          (pin (num "3") (name "GND") (type "power_in"))
          (pin (num "4") (name "Out") (type "output"))
          (pin (num "5") (name "VCC") (type "power_in"))))"#;

        let (i, part) = libpart(input).unwrap();
        assert_eq!(i, "");
        assert_eq!(part.part, "74LVC1G00");
        assert_eq!(part.pins.len(), 5);
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
