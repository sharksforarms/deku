use std::error;
use std::fmt;

/// Deku errors
#[derive(Debug, PartialEq)]
pub enum DekuError {
    /// Parsing error when reading
    Parse(String),
}

impl fmt::Display for DekuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DekuError::Parse(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl error::Error for DekuError {
    fn cause(&self) -> Option<&dyn error::Error> {
        Some(self)
    }
}
