//! # Read and manipulate KiCad netlist files
//!
//! The netlist is parsed from a provided `str` or `String` reference, and all data is stored as references into that string.

mod error;
mod parse;
mod parse_raw;
pub mod raw;
mod sexpr;

use std::collections::HashSet;

pub use error::NetListParseError;

/// The full netlist
#[derive(Debug, Clone)]
pub struct NetList<'a> {
    pub components: Vec<Component<'a>>,
    pub parts: Vec<Part<'a>>,
    pub nets: Vec<Net<'a>>,
}

/// Part identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartId<'a> {
    pub lib: &'a str,
    pub part: &'a str,
}

/// Reference designator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefDes<'a>(pub &'a str);

/// Pin number
///
/// Note that the number is a string, not an actual number, because we need to support, eg, BGA packages with pin numbers A1, A2, A3 etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PinNum<'a>(pub &'a str);

/// A component in the schematic
#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub ref_des: RefDes<'a>,
    pub value: &'a str,
    pub part_id: PartId<'a>,
    pub properties: Vec<(&'a str, &'a str)>,
    pub footprint: Option<&'a str>,
    pub pins: Vec<ComponentPin<'a>>,
}

/// The electrical type of the pin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A pin of an individual component
#[derive(Debug, Clone)]
pub struct ComponentPin<'a> {
    pub num: PinNum<'a>,
    pub name: &'a str,
    pub typ: PinType,
    pub net: &'a str,
}

/// A pin of a part
#[derive(Debug, Clone)]
pub struct PartPin<'a> {
    pub num: PinNum<'a>,
    pub name: &'a str,
    pub typ: PinType,
}

/// A part
#[derive(Debug, Clone)]
pub struct Part<'a> {
    pub part_id: PartId<'a>,
    pub description: &'a str,
    pub pins: Vec<PartPin<'a>>,
    pub components: Vec<RefDes<'a>>,
}

/// A node connects a net to a pin
#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub ref_des: RefDes<'a>,
    pub num: PinNum<'a>,
    pub function: Option<&'a str>,
    pub typ: PinType,
}

/// A net
#[derive(Debug, Clone)]
pub struct Net<'a> {
    /// A unique id for the net
    pub code: &'a str,
    pub name: &'a str,
    pub nodes: Vec<Node<'a>>,
}

impl<'a> TryFrom<&'a str> for NetList<'a> {
    type Error = NetListParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let raw: raw::NetList = value.try_into()?;
        raw.try_into()
    }
}

impl<'a> TryFrom<&'a String> for NetList<'a> {
    type Error = NetListParseError;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl<'a> NetList<'a> {
    pub fn parse(input: &'a str) -> Result<NetList<'a>, NetListParseError> {
        input.try_into()
    }

    /// Remove a component from the netlist
    pub fn remove_component(&mut self, ref_des: RefDes<'_>) {
        let Some(index) = self
            .components
            .iter()
            .position(|comp| comp.ref_des == ref_des)
        else {
            return;
        };

        let PartId { lib, part } = &self.components[index].part_id;
        let part_id = PartId { lib, part };

        self.components.remove(index);

        for net in self.nets.iter_mut() {
            net.nodes.retain(|node| node.ref_des != ref_des);
        }

        self.nets.retain(|net| !net.nodes.is_empty());

        if let Some(index) = self.parts.iter().position(|p| p.part_id == part_id) {
            self.parts[index].components.retain(|r| *r != ref_des);
            if self.parts[index].components.is_empty() {
                self.parts.remove(index);
            }
        }
    }

    /// Remove components from the netlist
    pub fn remove_components(&mut self, ref_des_list: &[RefDes<'_>]) {
        let removed_part_ids: HashSet<_> =
            HashSet::from_iter(self.components.iter().filter_map(|comp| {
                if ref_des_list.contains(&comp.ref_des) {
                    Some(comp.part_id)
                } else {
                    None
                }
            }));

        self.components
            .retain(|comp| !ref_des_list.contains(&comp.ref_des));

        for net in self.nets.iter_mut() {
            net.nodes
                .retain(|node| !ref_des_list.contains(&node.ref_des));
        }

        self.nets.retain(|net| !net.nodes.is_empty());

        for part_id in removed_part_ids {
            if let Some(index) = self.parts.iter().position(|p| p.part_id == part_id) {
                self.parts[index]
                    .components
                    .retain(|r| !ref_des_list.contains(r));
                if self.parts[index].components.is_empty() {
                    self.parts.remove(index);
                }
            }
        }
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
    fn remove_component_works() {
        let input = test_data!("kvt.net");
        let mut netlist: NetList = (&input).try_into().unwrap();

        assert_eq!(netlist.components.len(), 4);
        assert_eq!(netlist.parts.len(), 3);
        assert_eq!(netlist.nets.len(), 7);

        netlist.remove_component(RefDes("R1"));

        assert_eq!(netlist.components.len(), 3);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 7);

        netlist.remove_component(RefDes("U2"));

        assert_eq!(netlist.components.len(), 2);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 6);
    }

    #[test]
    fn remove_components_works() {
        let input = test_data!("kvt.net");
        let mut netlist: NetList = (&input).try_into().unwrap();

        assert_eq!(netlist.components.len(), 4);
        assert_eq!(netlist.parts.len(), 3);
        assert_eq!(netlist.nets.len(), 7);

        netlist.remove_components(&vec![RefDes("R1"), RefDes("U2")]);

        assert_eq!(netlist.components.len(), 2);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 6);
    }

    #[test]
    fn test_load_old_netlist() -> Result<(), NetListParseError> {
        let input = test_data!("old-vD.net");
        let _: NetList = (&input).try_into()?;
        Ok(())
    }
}
