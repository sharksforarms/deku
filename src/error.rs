//! Error module

#![cfg(feature = "alloc")]

use alloc::{format, string::String};

/// Number of bits needed to retry parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedSize {
    bits: usize,
}

impl NeedSize {
    /// Create new [NeedSize] from bits
    pub fn new(bits: usize) -> Self {
        Self { bits }
    }

    /// Number of bits needed
    pub fn bit_size(&self) -> usize {
        self.bits
    }

    /// Number of bytes needed
    pub fn byte_size(&self) -> usize {
        (self.bits + 7) / 8
    }
}

/// Deku errors
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DekuError {
    /// Parsing error when reading
    Incomplete(NeedSize),
    /// Parsing error when reading
    Parse(String),
    /// Invalid parameter
    InvalidParam(String),
    /// Unexpected error
    Unexpected(String),
    /// Assertion error from `assert` or `assert_eq` attributes
    Assertion(String),
    /// Could not resolve `id` for variant
    IdVariantNotFound,
}

impl From<core::num::TryFromIntError> for DekuError {
    fn from(e: core::num::TryFromIntError) -> DekuError {
        DekuError::Parse(format!("error parsing int: {}", e))
    }
}

impl From<core::array::TryFromSliceError> for DekuError {
    fn from(e: core::array::TryFromSliceError) -> DekuError {
        DekuError::Parse(format!("error parsing from slice: {}", e))
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
            DekuError::Incomplete(ref size) => write!(
                f,
                "Not enough data, need {} bits (or {} bytes)",
                size.bit_size(),
                size.byte_size()
            ),
            DekuError::Parse(ref err) => write!(f, "Parse error: {}", err),
            DekuError::InvalidParam(ref err) => write!(f, "Invalid param error: {}", err),
            DekuError::Unexpected(ref err) => write!(f, "Unexpected error: {}", err),
            DekuError::Assertion(ref err) => write!(f, "Assertion error: {}", err),
            DekuError::IdVariantNotFound => write!(f, "Could not resolve `id` for variant"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DekuError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(self)
    }
}
