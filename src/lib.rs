/*!
# Deku: Declarative binary reading and writing

Deriving a struct or enum with `DekuRead` and `DekuWrite` provides bit-level,
symmetric, serialization/deserialization implementations.

This allows the developer to focus on building and maintaining how the data is
represented and manipulated and not on redundant, error-prone, parsing/writing code.

This approach is especially useful when dealing with binary structures such as
TLVs or network protocols.

Under the hood, it makes use of the [bitvec](https://crates.io/crates/bitvec)
crate as the "Reader" and “Writer”

For documentation and examples on available `#[deku]` attributes and features,
see [attributes list](attributes)

For more examples, see the
[examples folder](https://github.com/sharksforarms/deku/tree/master/examples)!

## no_std

For use in `no_std` environments, `alloc` is the single feature which is required on deku.

# Example

Let's read big-endian data into a struct, with fields containing different sizes,
modify a value, and write it back

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct DekuTest {
    #[deku(bits = "4")]
    field_a: u8,
    #[deku(bits = "4")]
    field_b: u8,
    field_c: u16,
}

let mut data: Vec<u8> = vec![0b0110_1001, 0xBE, 0xEF];
let (_amt_read, mut val) = DekuTest::from_bytes((&mut data[..], 0)).unwrap();
assert_eq!(DekuTest {
    field_a: 0b0110,
    field_b: 0b1001,
    field_c: 0xBEEF,
}, val);

val.field_c = 0xC0FE;

let mut data_out = val.to_bytes().unwrap();
assert_eq!(vec![0b0110_1001, 0xC0, 0xFE], data_out);
```

# Composing

Deku structs/enums can be composed as long as they implement DekuRead / DekuWrite traits

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    header: DekuHeader,
    data: DekuData,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuHeader(u8);

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuData(u16);

let mut data: Vec<u8> = vec![0xAA, 0xEF, 0xBE];
let (_amt_read, mut val) = DekuTest::from_bytes((&mut data[..], 0)).unwrap();
assert_eq!(DekuTest {
    header: DekuHeader(0xAA),
    data: DekuData(0xBEEF),
}, val);

let mut data_out = val.to_bytes().unwrap();
assert_eq!(data, data_out);
```

# Vec

Vec<T> can be used in combination with the [count](attributes#count)
attribute (T must implement DekuRead/DekuWrite)

[bytes_read](attributes#bytes_read) or [bits_read](attributes#bits_read)
can also be used instead of `count` to read a specific size of each.


If the length of Vec changes, the original field specified in `count` will not get updated.
Calling `.update()` can be used to "update" the field!

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "self.data.len()")]
    count: u8,
    #[deku(count = "count")]
    data: Vec<u8>,
}

let mut data: Vec<u8> = vec![0x02, 0xBE, 0xEF, 0xFF, 0xFF];
let (_amt_read, mut val) = DekuTest::from_bytes((&mut data[..], 0)).unwrap();
assert_eq!(DekuTest {
    count: 0x02,
    data: vec![0xBE, 0xEF]
}, val);

let mut data_out = val.to_bytes().unwrap();
assert_eq!(vec![0x02, 0xBE, 0xEF], data_out);

// Pushing an element to data
val.data.push(0xAA);

assert_eq!(DekuTest {
    count: 0x02, // Note: this value has not changed
    data: vec![0xBE, 0xEF, 0xAA]
}, val);

let mut data_out = val.to_bytes().unwrap();
// Note: `count` is still 0x02 while 3 bytes got written
assert_eq!(vec![0x02, 0xBE, 0xEF, 0xAA], data_out);

// Use `update` to update `count`
val.update().unwrap();

assert_eq!(DekuTest {
    count: 0x03,
    data: vec![0xBE, 0xEF, 0xAA]
}, val);

```

# Enums

As enums can have multiple variants, each variant must have a way to match on
the incoming data.

First the "type" is read using the `type`, then is matched against the
variants given `id`. What happens after is the same as structs!

This is implemented with the [id](/attributes/index.html#id),
[id_pat](/attributes/index.html#id_pat), [default](/attributes/index.html#default) and
[type](attributes#type) attributes. See these for more examples.

If no `id` is specified, the variant will default to it's discriminant value.

If no variant can be matched and the `default` is not provided, a [DekuError::Parse](crate::error::DekuError)
error will be returned.

If no variant can be matched and the `default` is provided, a variant will be returned
based on the field marked with `default`.

Example:

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
enum DekuTest {
    #[deku(id = "0x01")]
    VariantA,
    #[deku(id = "0x02")]
    VariantB(u16),
}

let mut data: Vec<u8> = vec![0x01, 0x02, 0xEF, 0xBE];

let (amt_read, val) = DekuTest::from_bytes((&mut data[..], 0)).unwrap();
assert_eq!(DekuTest::VariantA , val);

let (amt_read, val) = DekuTest::from_bytes((&mut data[..], amt_read)).unwrap();
assert_eq!(DekuTest::VariantB(0xBEEF) , val);
```

# Context

Child parsers can get access to the parent's parsed values using the `ctx` attribute

For more information see [ctx attribute](attributes#ctx)

Example:

```rust
use deku::prelude::*;

#[derive(DekuRead, DekuWrite)]
#[deku(ctx = "a: u8")]
struct Subtype {
    #[deku(map = "|b: u8| -> Result<_, DekuError> { Ok(b + a) }")]
    b: u8
}

#[derive(DekuRead, DekuWrite)]
struct Root {
    a: u8,
    #[deku(ctx = "*a")] // `a` is a reference
    sub: Subtype
}

let mut data: Vec<u8> = vec![0x01, 0x02];

let (amt_read, value) = Root::from_bytes((&mut data[..], 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02)
```

# Internal variables and previously read fields

Along similar lines to [Context](#context) variables, previously read variables
are exposed and can be referenced:

Example:

```rust
# use deku::prelude::*;
#[derive(DekuRead)]
struct DekuTest {
    num_items: u8,
    #[deku(count = "num_items")]
    items: Vec<u16>,
}
```

The following variables are internals which can be used in attributes accepting
tokens such as `reader`, `writer`, `map`, `count`, etc.

These are provided as a convenience to the user.

Always included:
- `deku::container: &Container` - Current [`deku::container::Container`]
- `deku::input_bits: &BitSlice<u8, Msb0>` - The initial input in bits
- `deku::output: &mut BitSlice<u8, Msb0>` - The output bit stream

Conditionally included if referenced:
- `deku::bit_offset: usize` - Current bit offset from the input
- `deku::byte_offset: usize` - Current byte offset from the input

Example:
```rust
# use deku::prelude::*;
#[derive(DekuRead)]
#[deku(ctx = "size: u32")]
pub struct EncodedString {
    encoding: u8,

    #[deku(count = "size as usize - deku::byte_offset")]
    data: Vec<u8>
}
```

*/
#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unusual_byte_groupings)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// re-export of bitvec
pub mod acid_io {
    pub use acid_io::Cursor;
    pub use acid_io::Read;
}

/// re-export of bitvec
pub mod bitvec {
    pub use bitvec::prelude::*;
    pub use bitvec::view::BitView;
}

pub use deku_derive::*;

pub mod attributes;
pub mod container;
pub mod ctx;
pub mod error;
mod impls;
pub mod prelude;

pub use crate::error::DekuError;

/// "Read" trait: read bits and construct type
pub trait DekuRead<'a, Ctx = ()> {
    /// Read bits and construct type
    /// * **input** - Input as bits
    /// * **ctx** - A context required by context-sensitive reading. A unit type `()` means no context
    /// needed.
    ///
    /// Returns the amount of bits read after parsing in addition to Self.
    fn read(
        input: &'a bitvec::BitSlice<u8, bitvec::Msb0>,
        ctx: Ctx,
    ) -> Result<(usize, Self), DekuError>
    where
        Self: Sized;
}

/// "Reader" trait: read bytes and bits from [`Read`]er
pub trait DekuReader<'a, Ctx = ()> {
    /// Construct type from `container` implementing [`acid_io::Read`]
    fn from_reader<R: acid_io::Read>(
        container: &mut crate::container::Container<R>,
        ctx: Ctx,
    ) -> Result<Self, DekuError>
    where
        Self: Sized;
}

/// "Reader" trait: implemented on DekuRead struct and enum containers. A `container` is a type which
/// doesn't need any context information.
pub trait DekuContainerRead<'a>: DekuReader<'a, ()> {
    /// Read bytes and construct type
    /// * **input** - Input given as data and bit offset
    ///
    /// # Returns
    /// amount of bits read after parsing in addition to Self.
    fn from_bytes(input: (&'a mut [u8], usize)) -> Result<(usize, Self), DekuError>
    where
        Self: Sized;
}

/// "Writer" trait: write from type to bits
pub trait DekuWrite<Ctx = ()> {
    /// Write type to bits
    /// * **output** - Sink to store resulting bits
    /// * **ctx** - A context required by context-sensitive reading. A unit type `()` means no context
    /// needed.
    fn write(
        &self,
        output: &mut bitvec::BitVec<u8, bitvec::Msb0>,
        ctx: Ctx,
    ) -> Result<(), DekuError>;
}

/// "Writer" trait: implemented on DekuWrite struct and enum containers. A `container` is a type which
/// doesn't need any context information.
pub trait DekuContainerWrite: DekuWrite<()> {
    /// Write struct/enum to Vec<u8>
    fn to_bytes(&self) -> Result<Vec<u8>, DekuError>;

    /// Write struct/enum to BitVec
    fn to_bits(&self) -> Result<bitvec::BitVec<u8, bitvec::Msb0>, DekuError>;
}

/// "Updater" trait: apply mutations to a type
pub trait DekuUpdate {
    /// Apply updates
    fn update(&mut self) -> Result<(), DekuError>;
}

/// "Extended Enum" trait: obtain additional enum information
pub trait DekuEnumExt<'a, T> {
    /// Obtain `id` of a given enum variant
    fn deku_id(&self) -> Result<T, DekuError>;
}

/// Implements DekuWrite for references of types that implement DekuWrite
impl<T, Ctx> DekuWrite<Ctx> for &T
where
    T: DekuWrite<Ctx>,
    Ctx: Copy,
{
    /// Write value of type to bits
    fn write(
        &self,
        output: &mut bitvec::BitVec<u8, bitvec::Msb0>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        <T>::write(self, output, ctx)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/test_common/mod.rs"]
pub mod test_common;
