mod error;
mod parse;
mod sexpr;

pub use error::NetListParseError;

#[derive(Debug, Clone)]
pub struct NetList<'a> {
    pub components: Vec<Component<'a>>,
    pub parts: Vec<Part<'a>>,
    pub nets: Vec<Net<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartId<'a> {
    pub lib: &'a str,
    pub part: &'a str,
}

#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub reference: &'a str,
    pub value: &'a str,
    pub part_id: PartId<'a>,
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
    pub part_id: PartId<'a>,
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

impl<'a> NetList<'a> {
    pub fn remove_component(&mut self, reference: &str) {
        if let Some(index) = self
            .components
            .iter()
            .position(|comp| comp.reference == reference)
        {
            let comp = &self.components[index];

            for net in self.nets.iter_mut() {
                net.nodes.retain(|node| node.reference != reference);
            }

            self.nets.retain(|net| !net.nodes.is_empty());

            if self
                .components
                .iter()
                .filter(|c| c.part_id == comp.part_id)
                .count()
                == 1
            {
                if let Some(part_index) = self
                    .parts
                    .iter()
                    .position(|part| comp.part_id == part.part_id)
                {
                    self.parts.remove(part_index);
                }
            }

            self.components.remove(index);
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

        netlist.remove_component("R1");

        assert_eq!(netlist.components.len(), 3);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 7);

        netlist.remove_component("U2");

        assert_eq!(netlist.components.len(), 2);
        assert_eq!(netlist.parts.len(), 2);
        assert_eq!(netlist.nets.len(), 6);
    }
}
