use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected EOF at {at:?}")]
    UnexpectedEof { at: logos::Span },
    #[error("Expected {expected} but found {found}")]
    UnexpectedToken {
        expected: String,
        found: String,
        at: logos::Span,
    },
    #[error("Unexpected token {found} at {at:?}")]
    UnknownToken { found: String, at: logos::Span },
    #[error("SExpr {0} not found")]
    MissingChild(String),
    #[error("Value not found")]
    MissingValue(),
    #[error("Unknown pin type {0}")]
    UnknownPinType(String),
    #[error("Part {0} not found")]
    MissingPart(String),
    #[error("No net found for component {0}, pin {1}")]
    MissingNet(String, String),
    #[error("Unused part {0}")]
    UnusedPart(String),
    #[error("Unknown version {0}")]
    UnknownVersion(String),
}
