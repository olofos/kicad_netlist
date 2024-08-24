mod parser;

/// The full netlist
#[derive(Debug, Clone)]
pub struct NetList<'a> {
    pub components: Vec<Component<'a>>,
    pub parts: Vec<Part<'a>>,
    pub nets: Vec<Net<'a>>,
}

/// A component in the schematic
#[derive(Debug, Clone)]
pub struct Component<'a> {
    pub ref_des: &'a str,
    pub value: &'a str,
    pub part: &'a str,
    pub lib: &'a str,
    pub properties: Vec<(&'a str, &'a str)>,
    pub footprint: Option<&'a str>,
}

/// An indivudual pin
#[derive(Debug, Clone)]
pub struct Pin<'a> {
    pub num: &'a str,
    pub name: &'a str,
    pub typ: &'a str,
}

/// A part
#[derive(Debug, Clone)]
pub struct Part<'a> {
    pub part: &'a str,
    pub lib: &'a str,
    pub description: &'a str,
    pub pins: Vec<Pin<'a>>,
}

/// A node connects a net to a pin
#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub ref_des: &'a str,
    pub num: &'a str,
    pub function: Option<&'a str>,
    pub typ: &'a str,
}

/// A net
#[derive(Debug, Clone)]
pub struct Net<'a> {
    /// A unique id for the net
    pub code: &'a str,
    pub name: &'a str,
    pub nodes: Vec<Node<'a>>,
}
