use thiserror::Error;

/// Netlist parse errors
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
