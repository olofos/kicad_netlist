use crate::error::NetListParseError;
use crate::sexpr::{self, SExpr};
use crate::{Component, Net, NetList, Node, Part, PartId, Pin, PinNum, PinType, RefDes};

impl TryFrom<&str> for PinType {
    type Error = NetListParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.ends_with("no_connect") {
            return Ok(Self::Unconnected);
        }
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
            s => Err(NetListParseError::UnknownPinType(s.to_owned())),
        }
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Component<'a> {
    type Error = NetListParseError;
    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let ref_des = RefDes(value.value("ref")?);
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

        let part_id = PartId { lib, part };

        Ok(Self {
            ref_des,
            value: val,
            part_id,
            properties,
            footprint,
        })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Pin<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let num = PinNum(value.value("num")?);
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
        let part_id = PartId { lib, part };
        let description = value.value("description")?;
        let pins = if let Ok(pins) = value.child("pins") {
            pins.children("pin")
                .map(|pin| pin.try_into())
                .collect::<Result<_, _>>()?
        } else {
            vec![]
        };
        Ok(Part {
            part_id,
            description,
            pins,
        })
    }
}

impl<'a> TryFrom<&SExpr<'a>> for Node<'a> {
    type Error = NetListParseError;

    fn try_from(value: &SExpr<'a>) -> Result<Self, Self::Error> {
        let ref_des = RefDes(value.value("ref")?);
        let num = PinNum(value.value("pin")?);
        let function = value.value("pinfunction").ok();
        let typ = value.value("pintype")?.try_into()?;

        Ok(Node {
            ref_des,
            num,
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
        sexpr::parse(value)?.try_into()
    }
}

impl<'a> TryFrom<&'a String> for NetList<'a> {
    type Error = NetListParseError;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
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
    fn can_parse_comp() {
        let i = &test_data!("kvt.net");
        let root = sexpr::parse(i).unwrap();

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
        let root = sexpr::parse(i).unwrap();

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
        let root = sexpr::parse(i).unwrap();

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
        let root = sexpr::parse(i).unwrap();
        let netlist: NetList = root.try_into().unwrap();

        assert_eq!(netlist.components.len(), 4);
        assert_eq!(netlist.parts.len(), 3);
        assert_eq!(netlist.nets.len(), 7);
    }
}
