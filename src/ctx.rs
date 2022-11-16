//! Types for context representation
//! See [ctx attribute](super::attributes#ctx) for more information.

use core::marker::PhantomData;
use core::str::FromStr;

/// Aligned and correctly padded bytes
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Aligned;

/// An endian
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Endian {
    /// Little endian
    Little,
    /// Big endian
    Big,
}

/// Error returned when parsing a `Endian` using [`from_str`]
///
/// [`from_str`]: Endian::from_str()
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEndianError {}

impl Endian {
    /// [`Endian::default`], but const.
    ///
    /// [`Endian::default`]: Endian::default()
    pub const fn new() -> Self {
        #[cfg(target_endian = "little")]
        let endian = Endian::Little;

        #[cfg(target_endian = "big")]
        let endian = Endian::Big;

        endian
    }

    /// Is it little endian
    pub fn is_le(self) -> bool {
        self == Endian::Little
    }

    /// Is it big endian
    pub fn is_be(self) -> bool {
        self == Endian::Big
    }
}

impl Default for Endian {
    /// Return the endianness of the target's CPU.
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Endian {
    type Err = ParseEndianError;

    /// Parse a `Endian` from a string.
    /// # Examples
    /// ```rust
    /// use std::str::FromStr;
    /// use deku::ctx::Endian;
    /// assert_eq!(FromStr::from_str("little"), Ok(Endian::Little));
    /// assert_eq!(FromStr::from_str("big"), Ok(Endian::Big));
    /// assert!(<Endian as FromStr>::from_str("not an endian").is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "little" => Ok(Endian::Little),
            "big" => Ok(Endian::Big),
            _ => Err(ParseEndianError {}),
        }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
// derive_partial_eq_without_eq false positive in struct using traits
// For details: https://github.com/rust-lang/rust-clippy/issues/9413
/// A limit placed on a container's elements
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum Limit<T, Predicate: FnMut(&T) -> bool> {
    /// Read a specific count of elements
    Count(usize),

    /// Read until a given predicate holds true
    Until(Predicate, PhantomData<T>),

    /// Read until a given quantity of bytes have been read
    ByteSize(ByteSize),

    /// Read until a given quantity of bits have been read
    BitSize(BitSize),
}

impl<T> From<usize> for Limit<T, fn(&T) -> bool> {
    fn from(n: usize) -> Self {
        Limit::Count(n)
    }
}

impl<T, Predicate: for<'a> FnMut(&'a T) -> bool> From<Predicate> for Limit<T, Predicate> {
    fn from(predicate: Predicate) -> Self {
        Limit::Until(predicate, PhantomData)
    }
}

impl<T> From<ByteSize> for Limit<T, fn(&T) -> bool> {
    fn from(size: ByteSize) -> Self {
        Limit::ByteSize(size)
    }
}

impl<T> From<BitSize> for Limit<T, fn(&T) -> bool> {
    fn from(size: BitSize) -> Self {
        Limit::BitSize(size)
    }
}

impl<T, Predicate: for<'a> FnMut(&'a T) -> bool> Limit<T, Predicate> {
    /// Constructs a new Limit that reads until the given predicate returns true
    /// The predicate is given a reference to the latest read value and must return
    /// true to stop reading
    pub fn new_until(predicate: Predicate) -> Self {
        predicate.into()
    }
}

impl<T> Limit<T, fn(&T) -> bool> {
    /// Constructs a new Limit that reads until the given number of elements are read
    pub fn new_count(count: usize) -> Self {
        count.into()
    }

    /// Constructs a new Limit that reads until the given size
    pub fn new_bit_size(size: BitSize) -> Self {
        size.into()
    }

    /// Constructs a new Limit that reads until the given size
    pub fn new_byte_size(size: ByteSize) -> Self {
        size.into()
    }
}

/// The size of field in bytes
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ByteSize(pub usize);

/// The size of field in bits
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct BitSize(pub usize);

impl BitSize {
    /// Convert the size in bytes to a bit size.
    const fn bits_from_bytes(byte_size: usize) -> Self {
        // TODO: use checked_mul when const_option is enabled
        // link: https://github.com/rust-lang/rust/issues/67441
        Self(byte_size * 8)
    }

    /// Returns the bit size of a type.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::BitSize;
    ///
    /// assert_eq!(BitSize::of::<i32>(), BitSize(4 * 8));
    /// ```
    pub const fn of<T>() -> Self {
        Self::bits_from_bytes(core::mem::size_of::<T>())
    }

    /// Returns the bit size of the pointed-to value
    pub fn of_val<T: ?Sized>(val: &T) -> Self {
        Self::bits_from_bytes(core::mem::size_of_val(val))
    }
}
