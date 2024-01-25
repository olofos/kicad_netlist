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

#[cfg(test)]
mod tests {}
