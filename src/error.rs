//! Error module

#[cfg(feature = "alloc")]
use alloc::borrow::Cow;

use no_std_io::io::ErrorKind;

/// Number of bits needed to retry parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedSize {
    bits: usize,
}

impl NeedSize {
    /// Create new [NeedSize] from bits
    #[inline]
    pub fn new(bits: usize) -> Self {
        Self { bits }
    }

    /// Number of bits needed
    #[inline]
    pub fn bit_size(&self) -> usize {
        self.bits
    }

    /// Number of bytes needed
    #[inline]
    pub fn byte_size(&self) -> usize {
        self.bits.div_ceil(8)
    }
}

#[cfg(feature = "alloc")]
type DekuErrorString = Cow<'static, str>;

#[cfg(not(feature = "alloc"))]
type DekuErrorString = &'static str;

/// Deku errors
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DekuError {
    /// Parsing error when reading
    Incomplete(NeedSize),
    /// Parsing error when reading
    Parse(DekuErrorString),
    /// Invalid parameter
    InvalidParam(DekuErrorString),
    /// Assertion error from `assert` or `assert_eq` attributes
    Assertion(DekuErrorString),
    /// Assertion error from `assert` or `assert_eq` attributes, without string
    AssertionNoStr,
    /// Could not resolve `id` for variant
    IdVariantNotFound,
    /// IO error while reading or writing
    Io(ErrorKind),
}

/// Abstract over alloc vs no-alloc for handling of error strings
///
/// The macro takes the DekuError variant as an argument and performs the
/// instantiation itself in order to cater to the no-alloc case where we discard
/// all arguments beyond the description.
///
/// Invoked like:
///
/// ```
/// use deku::{deku_error, DekuError};
/// # extern crate alloc;
/// # let bit_size = 5;
/// # let input_size = 4;
/// let _ = deku_error!(DekuError::Parse,
///                     "bit size is greater than input size",
///                     "{} exceeds {}",
///                     bit_size,
///                     input_size);
/// ```
#[cfg(feature = "alloc")]
#[macro_export]
macro_rules! deku_error {
    ($p:path, $desc:expr, $fmt:expr, $($arg:expr),*) => {{
        extern crate alloc;
        $p(alloc::borrow::Cow::from(alloc::format!(concat!($desc, ": ", $fmt), $($arg),*)))
    }};
    ($p:path, $desc:expr, $fmt:expr) => {{
        extern crate alloc;
        $p(alloc::borrow::Cow::from(alloc::format!(concat!($desc, ": ", $fmt))))
    }};
    ($p:path, $desc:expr) => {{
        extern crate alloc;
        $p(alloc::borrow::Cow::from($desc))
    }};
    ($p:path) => {{ $p }};
}

/// Abstract over alloc vs no-alloc for handling of error strings
///
/// The macro takes the DekuError variant as an argument and performs the
/// instantiation itself in order to cater to the no-alloc case where we discard
/// all arguments beyond the description.
///
/// Invoked like:
///
/// ```
/// use deku::{deku_error, DekuError};
/// # let bit_size = 5;
/// # let input_size = 4;
/// let _ = deku_error!(DekuError::Parse,
///                     "bit size is greater than input size",
///                     "{} exceeds {}",
///                     bit_size,
///                     input_size);
/// ```
#[cfg(not(feature = "alloc"))]
#[macro_export]
macro_rules! deku_error {
    ($p:path, $desc:expr, $fmt:expr, $($arg:expr),*) => {{
        $(let _ = $arg;)*
        $p($desc)
    }};
    ($p:path, $desc:expr, $fmt:expr) => {{
        $p($desc)
    }};
    ($p:path, $desc:expr) => {{
        $p($desc)
    }};
    ($p:path) => {{ $p }};
}

impl From<core::num::TryFromIntError> for DekuError {
    fn from(e: core::num::TryFromIntError) -> DekuError {
        deku_error!(DekuError::Parse, "error parsing int", "{}", e)
    }
}

impl From<core::array::TryFromSliceError> for DekuError {
    fn from(e: core::array::TryFromSliceError) -> DekuError {
        deku_error!(DekuError::Parse, "error parsing from slice", "{}", e)
    }
}

impl From<core::convert::Infallible> for DekuError {
    fn from(_e: core::convert::Infallible) -> DekuError {
        unreachable!();
    }
}

#[cfg(feature = "std")]
impl core::fmt::Display for DekuError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            DekuError::Incomplete(ref size) => write!(
                f,
                "Not enough data, need {} bits (or {} bytes)",
                size.bit_size(),
                size.byte_size()
            ),
            DekuError::Parse(ref err) => write!(f, "Parse error: {err}"),
            DekuError::InvalidParam(ref err) => write!(f, "Invalid param error: {err}"),
            DekuError::Assertion(ref err) => write!(f, "Assertion error: {err}"),
            DekuError::AssertionNoStr => write!(f, "Assertion error"),
            DekuError::IdVariantNotFound => write!(f, "Could not resolve `id` for variant"),
            DekuError::Io(ref e) => write!(f, "io errorr: {e:?}"),
        }
    }
}

#[cfg(not(feature = "std"))]
impl core::error::Error for DekuError {}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for DekuError {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl From<DekuError> for no_std_io::io::Error {
    fn from(error: DekuError) -> Self {
        use no_std_io::io;
        match error {
            DekuError::Incomplete(_) => {
                io::Error::new(io::ErrorKind::UnexpectedEof, "Short message buffer")
            }
            DekuError::Parse(msg) => io::Error::new(io::ErrorKind::InvalidData, msg),
            DekuError::InvalidParam(msg) => io::Error::new(io::ErrorKind::InvalidInput, msg),
            DekuError::Assertion(msg) => io::Error::new(io::ErrorKind::InvalidData, msg),
            DekuError::IdVariantNotFound => {
                io::Error::new(io::ErrorKind::NotFound, "Variant not found for ID")
            }
            DekuError::Io(kind) => io::Error::new(kind, "IO failure during parsing"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DekuError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(self)
    }
}

#[cfg(feature = "std")]
impl From<DekuError> for std::io::Error {
    fn from(error: DekuError) -> Self {
        use std::io;
        match error {
            DekuError::Incomplete(_) => io::Error::new(io::ErrorKind::UnexpectedEof, error),
            DekuError::Parse(_) => io::Error::new(io::ErrorKind::InvalidData, error),
            DekuError::InvalidParam(_) => io::Error::new(io::ErrorKind::InvalidInput, error),
            DekuError::Assertion(_) => io::Error::new(io::ErrorKind::InvalidData, error),
            DekuError::AssertionNoStr => io::Error::from(io::ErrorKind::InvalidData),
            DekuError::IdVariantNotFound => io::Error::new(io::ErrorKind::NotFound, error),
            DekuError::Io(e) => io::Error::new(e, error),
        }
    }
}
