use std::error;
use std::fmt;

/// Deku errors
#[derive(Debug, PartialEq)]
pub enum DekuError {
    /// Parsing error when reading
    Parse(String),
    /// Invalid parameter
    InvalidParam(String),
}

impl From<std::num::TryFromIntError> for DekuError {
    fn from(e: std::num::TryFromIntError) -> DekuError {
        DekuError::Parse(format!("error parsing int: {}", e.to_string()))
    }
}

impl From<std::convert::Infallible> for DekuError {
    fn from(_e: std::convert::Infallible) -> DekuError {
        unreachable!();
    }
}

impl fmt::Display for DekuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DekuError::Parse(ref err) => write!(f, "Parse error: {}", err),
            DekuError::InvalidParam(ref err) => write!(f, "Invalid param error: {}", err),
        }
    }
}

impl error::Error for DekuError {
    fn cause(&self) -> Option<&dyn error::Error> {
        Some(self)
    }
}
