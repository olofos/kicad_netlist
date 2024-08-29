//! # Read and manipulate KiCad netlist files
//!
//! The netlist is parsed from a provided `str` or `String` reference, and all data is stored as references into that string.

mod error;
mod parse;
pub mod raw;
mod sexpr;

use std::collections::HashSet;

pub use error::ParseError;

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

/// General property
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Property<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

/// Define simple wrapper types
macro_rules! define_pub_str_wrapper {
    ($name:ident,$doc:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[doc = $doc]
        pub struct $name<'a>(&'a str);

        impl std::fmt::Display for $name<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl<'a> $name<'a> {
            pub fn as_str(&self) -> &str {
                self.0.as_ref()
            }
        }

        impl<'a> From<&'a str> for $name<'a> {
            fn from(value: &'a str) -> Self {
                Self(value.into())
            }
        }
    };
}

define_pub_str_wrapper!(RefDes, "Reference designator");
define_pub_str_wrapper!(PinNum, "Pin number\n\nNote that the number is a string, not an actual number, because we need to support, eg, BGA packages with pin numbers A1, A2, A3 etc.");
define_pub_str_wrapper!(PinName, "Name of pin");
define_pub_str_wrapper!(PinFunction, "Pin function");
define_pub_str_wrapper!(Value, "Component value");
define_pub_str_wrapper!(Footprint, "Footprint");
define_pub_str_wrapper!(NetName, "Name of net");
define_pub_str_wrapper!(NetCode, "Net id");
define_pub_str_wrapper!(PartDescription, "Description");

/// A component in the schematic
#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub ref_des: RefDes<'a>,
    pub value: Value<'a>,
    pub part_id: PartId<'a>,
    pub properties: Vec<Property<'a>>,
    pub footprint: Option<Footprint<'a>>,
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
    pub name: PinName<'a>,
    pub typ: PinType,
    pub net: NetName<'a>,
}

/// A pin of a part
#[derive(Debug, Clone)]
pub struct PartPin<'a> {
    pub num: PinNum<'a>,
    pub name: PinName<'a>,
    pub typ: PinType,
}

/// A part
#[derive(Debug, Clone)]
pub struct Part<'a> {
    pub part_id: PartId<'a>,
    pub description: PartDescription<'a>,
    pub pins: Vec<PartPin<'a>>,
    pub components: Vec<RefDes<'a>>,
}

/// A node connects a net to a pin
#[derive(Debug, Clone)]
pub struct NetNode<'a> {
    pub ref_des: RefDes<'a>,
    pub num: PinNum<'a>,
    pub function: Option<PinFunction<'a>>,
    pub typ: PinType,
}

/// A net
#[derive(Debug, Clone)]
pub struct Net<'a> {
    /// A unique id for the net
    pub code: NetCode<'a>,
    pub name: NetName<'a>,
    pub nodes: Vec<Node<'a>>,
}

impl<'a> TryFrom<&'a str> for NetList<'a> {
    type Error = ParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let raw: raw::NetList = value.try_into()?;
        raw.try_into()
    }
}

impl<'a> TryFrom<&'a String> for NetList<'a> {
    type Error = ParseError;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl<'a> NetList<'a> {
    pub fn parse(input: &'a str) -> Result<NetList<'a>, ParseError> {
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

        let part_id = self.components[index].part_id.clone();

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
                    Some(comp.part_id.clone())
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

        netlist.remove_component(RefDes::from("R1"));

        assert_eq!(netlist.components.len(), 3);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 7);

        netlist.remove_component(RefDes::from("U2"));

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

        netlist.remove_components(&vec![RefDes::from("R1"), RefDes::from("U2")]);

        assert_eq!(netlist.components.len(), 2);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 6);
    }

    #[test]
    fn test_load_old_netlist() -> Result<(), ParseError> {
        let input = test_data!("old-vD.net");
        let result: Result<NetList, _> = (&input).try_into();
        match result {
            Err(ParseError::UnknownVersion(version)) if version == "D" => Ok(()),
            Err(err) => Err(err),
            Ok(_) => panic!("Expected an error"),
        }
    }
}
