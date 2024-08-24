use crate::{
    raw, Component, ComponentPin, Net, NetList, Node, ParseError, Part, PartId, PartPin, PinNum,
    PinType, RefDes,
};

impl TryFrom<&str> for PinType {
    type Error = ParseError;

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
            s => Err(ParseError::UnknownPinType(s.to_owned())),
        }
    }
}

impl<'a> TryFrom<raw::Pin<'a>> for PartPin<'a> {
    type Error = ParseError;

    fn try_from(value: raw::Pin<'a>) -> Result<Self, Self::Error> {
        let raw::Pin { num, name, typ } = value;
        let num = PinNum(num);
        let typ = typ.try_into()?;
        Ok(PartPin { num, name, typ })
    }
}

impl<'a> TryFrom<raw::Part<'a>> for Part<'a> {
    type Error = ParseError;

    fn try_from(value: raw::Part<'a>) -> Result<Self, Self::Error> {
        let raw::Part {
            part,
            lib,
            description,
            pins,
        } = value;
        let part_id = PartId { lib, part };
        let pins = pins
            .into_iter()
            .map(|pin| pin.try_into())
            .collect::<Result<_, _>>()?;
        Ok(Part {
            part_id,
            description,
            pins,
            components: vec![],
        })
    }
}

impl<'a> TryFrom<raw::Component<'a>> for Component<'a> {
    type Error = ParseError;

    fn try_from(value: raw::Component<'a>) -> Result<Self, Self::Error> {
        let raw::Component {
            ref_des,
            value,
            part,
            lib,
            properties,
            footprint,
        } = value;
        let ref_des = RefDes(ref_des);
        let part_id = PartId { lib, part };

        Ok(Component {
            ref_des,
            value,
            part_id,
            properties,
            footprint,
            pins: vec![],
        })
    }
}

impl<'a> TryFrom<raw::Node<'a>> for Node<'a> {
    type Error = ParseError;

    fn try_from(value: raw::Node<'a>) -> Result<Self, Self::Error> {
        let raw::Node {
            ref_des,
            num,
            function,
            typ,
        } = value;
        let ref_des = RefDes(ref_des);
        let num = PinNum(num);
        let typ = typ.try_into()?;
        Ok(Node {
            ref_des,
            num,
            function,
            typ,
        })
    }
}

impl<'a> TryFrom<raw::Net<'a>> for Net<'a> {
    type Error = ParseError;

    fn try_from(value: raw::Net<'a>) -> Result<Self, Self::Error> {
        let raw::Net { code, name, nodes } = value;
        let nodes = nodes
            .into_iter()
            .map(|node| node.try_into())
            .collect::<Result<_, _>>()?;
        Ok(Net { code, name, nodes })
    }
}

impl<'a> TryFrom<raw::NetList<'a>> for NetList<'a> {
    type Error = ParseError;

    fn try_from(value: raw::NetList<'a>) -> Result<Self, Self::Error> {
        let raw::NetList {
            components,
            parts,
            nets,
        } = value;

        let mut components: Vec<Component> = components
            .into_iter()
            .map(|comp| comp.try_into())
            .collect::<Result<_, _>>()?;

        let mut parts: Vec<Part> = parts
            .into_iter()
            .map(|part| part.try_into())
            .collect::<Result<_, _>>()?;

        let nets: Vec<Net> = nets
            .into_iter()
            .map(|net| net.try_into())
            .collect::<Result<_, _>>()?;

        for comp in components.iter_mut() {
            let part =
                parts
                    .iter()
                    .find(|p| p.part_id == comp.part_id)
                    .ok_or(ParseError::MissingPart(format!(
                        "{}/{}",
                        comp.part_id.lib, comp.part_id.part
                    )))?;
            comp.pins = part
                .pins
                .iter()
                .map(|pin| {
                    let PartPin { num, name, typ } = pin;
                    let num = *num;
                    let typ = *typ;
                    let net = nets
                        .iter()
                        .find(|net| {
                            net.nodes
                                .iter()
                                .any(|node| node.ref_des == comp.ref_des && node.num == num)
                        })
                        .ok_or(ParseError::MissingNet(
                            comp.ref_des.0.to_string(),
                            num.0.to_string(),
                        ))?;
                    let net = net.name;
                    Ok(ComponentPin {
                        num,
                        name,
                        typ,
                        net,
                    })
                })
                .collect::<Result<_, ParseError>>()?;
        }

        for part in parts.iter_mut() {
            part.components = components
                .iter()
                .filter_map(|comp| {
                    if comp.part_id == part.part_id {
                        Some(comp.ref_des)
                    } else {
                        None
                    }
                })
                .collect();
            if part.components.is_empty() {
                return Err(ParseError::UnusedPart(format!(
                    "{}/{}",
                    part.part_id.lib, part.part_id.part
                )));
            }
        }

        Ok(NetList {
            components,
            parts,
            nets,
        })
    }
}
