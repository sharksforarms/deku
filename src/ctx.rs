//! Types for context representation
//! See [ctx attribute](super::attributes#ctx) for more information.

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::str::FromStr;

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

/// A limit placed on a container's elements
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Limit<T, Predicate: FnMut(&T) -> bool> {
    /// Read a specific count of elements
    Count(usize),

    /// Read until a given predicate holds true
    Until(Predicate, PhantomData<T>),

    /// Read until a given quantity of bits have been read
    Bits(BitSize),
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

impl<T> From<BitSize> for Limit<T, fn(&T) -> bool> {
    fn from(bits: BitSize) -> Self {
        Limit::Bits(bits)
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
    /// Constructs a new Limit that reads unil the given number of elements are read
    pub fn new_count(count: usize) -> Self {
        count.into()
    }

    /// Constructs a new Limit that reads until the given number of bits have been read
    pub fn new_bits(bits: BitSize) -> Self {
        bits.into()
    }
}

/// The number bits in a field
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct BitSize(pub usize);

impl BitSize {
    /// Convert the size in bytes to a bit size.
    /// # Examples
    /// ```rust
    /// # use std::mem::size_of;
    /// # use deku::ctx::BitSize;
    ///
    /// assert_eq!(BitSize::with_byte_size(1), BitSize(8));
    /// ```
    ///
    /// # Panic
    /// Panic if `byte_size * 8` is greater than `usize::MAX`.
    pub fn with_byte_size(byte_size: usize) -> Self {
        Self(byte_size.checked_mul(8).expect("bit size overflow"))
    }

    /// Returns the bit size of a type.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::BitSize;
    ///
    /// assert_eq!(BitSize::of::<i32>(), BitSize(4 * 8));
    /// ```
    /// # Panics
    /// Panic if the bit size of given type is greater than `usize::MAX`
    pub fn of<T>() -> Self {
        Self::with_byte_size(core::mem::size_of::<T>())
    }

    /// Returns the bit size of the pointed-to value
    pub fn of_val<T: ?Sized>(val: &T) -> Self {
        Self::with_byte_size(core::mem::size_of_val(val))
    }
}

impl Into<usize> for BitSize {
    fn into(self) -> usize {
        self.0
    }
}

impl From<usize> for BitSize {
    fn from(n: usize) -> Self {
        Self(n)
    }
}

impl Deref for BitSize {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BitSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
