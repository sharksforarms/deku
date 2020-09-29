//! Error module

#![cfg(feature = "alloc")]

use alloc::{format, string::String, string::ToString};

/// Deku errors
#[derive(Debug, PartialEq)]
pub enum DekuError {
    /// Parsing error when reading
    Parse(String),
    /// Invalid parameter
    InvalidParam(String),
    /// Unexpected error
    Unexpected(String),
}

impl From<core::num::TryFromIntError> for DekuError {
    fn from(e: core::num::TryFromIntError) -> DekuError {
        DekuError::Parse(format!("error parsing int: {}", e.to_string()))
    }
}

impl From<core::array::TryFromSliceError> for DekuError {
    fn from(e: core::array::TryFromSliceError) -> DekuError {
        DekuError::Parse(format!("error parsing from slice: {}", e.to_string()))
    }
}

impl From<core::convert::Infallible> for DekuError {
    fn from(_e: core::convert::Infallible) -> DekuError {
        unreachable!();
    }
}

impl core::fmt::Display for DekuError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            DekuError::Parse(ref err) => write!(f, "Parse error: {}", err),
            DekuError::InvalidParam(ref err) => write!(f, "Invalid param error: {}", err),
            DekuError::Unexpected(ref err) => write!(f, "Unexpected error: {}", err),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DekuError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(self)
    }
}
