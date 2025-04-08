/*!

# Deku: Declarative binary reading and writing

Deriving a struct or enum with `DekuRead` and `DekuWrite` provides bit-level,
symmetric, serialization/deserialization implementations.

This allows the developer to focus on building and maintaining how the data is
represented and manipulated and not on redundant, error-prone, parsing/writing code.
This approach is especially useful when dealing with binary structures such as
TLVs or network protocols. This allows the internal rustc compiler to choose
the in-memory representation of the struct, while reading and writing can
understand the struct in a "packed" C way.

Under the hood, many specializations are done in order to achieve performant code.
For reading and writing bytes, the std library is used.
When bit-level control is required, it makes use of the [bitvec](https://crates.io/crates/bitvec)
crate as the "Reader" and “Writer”.

For documentation and examples on available `#[deku]` attributes and features,
see [attributes list](attributes)

For more examples, see the
[examples folder](https://github.com/sharksforarms/deku/tree/master/examples)!

## no_std

For use in `no_std` environments, `alloc` is the single feature which is required on deku.

# Example

Let's read big-endian data into a struct, with fields containing different sizes,
modify a value, and write it back. In this example we use [from_bytes](DekuContainerRead::from_bytes),
but we could also use [from_reader](DekuContainerRead::from_reader).
```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct DekuTest {
    #[deku(bits = 4)]
    field_a: u8,
    #[deku(bits = 4)]
    field_b: u8,
    field_c: u16,
}

let data: Vec<u8> = vec![0b0110_1001, 0xBE, 0xEF];
let (_rest, mut val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
assert_eq!(DekuTest {
    field_a: 0b0110,
    field_b: 0b1001,
    field_c: 0xBEEF,
}, val);

val.field_c = 0xC0FE;

let data_out = val.to_bytes().unwrap();
assert_eq!(vec![0b0110_1001, 0xC0, 0xFE], data_out);
```

# Composing

Deku structs/enums can be composed as long as they implement [DekuReader] / [DekuWrite] traits which
can be derived by using the `DekuRead` and `DekuWrite` Derive macros.

```rust
# use std::io::Cursor;
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

let data: Vec<u8> = vec![0xAA, 0xEF, 0xBE];
let (_rest, mut val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
assert_eq!(DekuTest {
    header: DekuHeader(0xAA),
    data: DekuData(0xBEEF),
}, val);

let data_out = val.to_bytes().unwrap();
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
# use std::io::Cursor;
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "self.data.len()")]
    count: u8,
    #[deku(count = "count")]
    data: Vec<u8>,
}

let data: Vec<u8> = vec![0x02, 0xBE, 0xEF, 0xFF, 0xFF];
let (_rest, mut val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
assert_eq!(DekuTest {
    count: 0x02,
    data: vec![0xBE, 0xEF]
}, val);

let data_out = val.to_bytes().unwrap();
assert_eq!(vec![0x02, 0xBE, 0xEF], data_out);

// Pushing an element to data
val.data.push(0xAA);

assert_eq!(DekuTest {
    count: 0x02, // Note: this value has not changed
    data: vec![0xBE, 0xEF, 0xAA]
}, val);

let data_out = val.to_bytes().unwrap();
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

First the "type" is read using `id_type`, then is matched against the
variants given `id`. What happens after is the same as structs!

This is implemented with the [id](attributes#id),
[id_pat](attributes#id_pat), [default](attributes#default) and
[id_type](attributes#id_type) attributes. See these for more examples.

If no `id` is specified, the variant will default to it's discriminant value.

If no variant can be matched and the `default` is not provided, a [DekuError::Parse](crate::error::DekuError)
error will be returned.

If no variant can be matched and the `default` is provided, a variant will be returned
based on the field marked with `default`.

Example:

```rust
# use std::io::Cursor;
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id = 0x01)]
    VariantA,
    #[deku(id = 0x02)]
    VariantB(u16),
}

let data: &[u8] = &[0x01, 0x02, 0xEF, 0xBE];
let mut cursor = Cursor::new(data);

let (_, val) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
assert_eq!(DekuTest::VariantA , val);

// cursor now points at 0x02
let (_, val) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
assert_eq!(DekuTest::VariantB(0xBEEF) , val);
```

# Context

Child parsers can get access to the parent's parsed values using the `ctx` attribute

For more information see [ctx attribute](attributes#ctx)

Example:

```rust
# use std::io::Cursor;
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

let data: &[u8] = &[0x01, 0x02];
let mut cursor = Cursor::new(data);

let (amt_read, value) = Root::from_reader((&mut cursor, 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02)
```

# `Read` supported
Parsers can be created that directly read from a source implementing [Read](crate::no_std_io::Read).

The crate [no_std_io2](https://crates.io/crates/no-std-io2) is re-exported as [no_std_io] for use in `no_std` environments.
This functions as an alias for [std::io](https://doc.rust-lang.org/stable/std/io/) when not
using `no_std`.

```rust, no_run
# use std::io::{Seek, SeekFrom, Read};
# use std::fs::File;
# use deku::prelude::*;
#[derive(Debug, DekuRead, DekuWrite, PartialEq, Eq, Clone)]
#[deku(endian = "big")]
struct EcHdr {
    magic: [u8; 4],
    version: u8,
    padding1: [u8; 3],
}

let mut file = File::options().read(true).open("file").unwrap();
let ec = EcHdr::from_reader((&mut file, 0)).unwrap();
```

# `Write` supported
Parsers can be created that directly write to a source implementing [Write](crate::no_std_io::Write).

```rust, no_run
# use std::io::{Seek, SeekFrom, Read};
# use std::fs::File;
# use deku::prelude::*;
#[derive(Debug, DekuRead, DekuWrite, PartialEq, Eq, Clone)]
#[deku(endian = "big")]
struct Hdr {
    version: u8,
}
let hdr = Hdr { version: 0xf0 };
let mut file = File::options().write(true).open("file").unwrap();
hdr.to_writer(&mut Writer::new(file), ());
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
- `deku::reader: &mut Reader` - Current [Reader]
- `deku::writer: &mut Writer` - Current [Writer]

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

# Debugging decoders with the `logging` feature.

If you are having trouble understanding what causes a Deku parse error, you may find the `logging`
feature useful.

To use it, you will need to:
 - enable the `logging` Cargo feature for your Deku dependency
 - import the `log` crate and a compatible logging library

For example, to log with `env_logger`, the dependencies in your `Cargo.toml` might look like:

```text
deku = { version = "*", features = ["logging"] }
log = "*"
env_logger = "*"
```

Then you'd call `env_logger::init()` or `env_logger::try_init()` prior to doing Deku decoding.

Deku uses the `trace` logging level, so if you run your application with `RUST_LOG=trace` in your
environment, you will see logging messages as Deku does its deserialising.

# Reducing parser code size

- With the use of the `no-assert-string` feature, you can remove the strings Deku adds to assertion errors.
- `DekuError` whenever possible will use a `'static str`, to make the errors compile away when following a
  guide such as [min-sized-rust](https://github.com/johnthagen/min-sized-rust).

# Performance: Compile without `bitvec`
The feature `bits` enables the `bitvec` crate to use when reading and writing, which is enabled by default.
This however slows down the reading and writing process if your code doesn't use `bits` and the `bit_offset`
in `from_bytes`.

# NoSeek
Unseekable streams such as [TcpStream](https://doc.rust-lang.org/std/net/struct.TcpStream.html) are supported through the [NoSeek](noseek::NoSeek) wrapper.

*/
#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unusual_byte_groupings)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// re-export of [no_std_io2](https://crates.io/crates/no-std-io2)
pub mod no_std_io {
    pub use no_std_io::io::Cursor;
    pub use no_std_io::io::Read;
    pub use no_std_io::io::Result;
    pub use no_std_io::io::Seek;
    pub use no_std_io::io::SeekFrom;
    pub use no_std_io::io::Write;
}

/// re-export of bitvec
#[cfg(feature = "bits")]
pub mod bitvec {
    pub use bitvec::prelude::*;
    pub use bitvec::view::BitView;
}

pub use deku_derive::*;

pub mod attributes;
pub mod ctx;
pub mod error;
mod impls;
pub mod noseek;
pub mod prelude;
pub mod reader;
pub mod writer;

pub use crate::error::DekuError;
use crate::reader::Reader;
use crate::writer::Writer;

/// "Reader" trait: read bytes and bits from [`no_std_io::Read`]er
#[rustversion::attr(
    since(1.78),
    diagnostic::on_unimplemented(
        note = "implement by adding #[derive(DekuRead)] to `{Self}`",
        note = "make sure the `ctx` sent into the function matches `{Self}`'s `ctx`",
    )
)]
pub trait DekuReader<'a, Ctx = ()> {
    /// Construct type from `reader` implementing [`no_std_io::Read`], with ctx.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use std::io::{Seek, SeekFrom, Read};
    /// # use std::fs::File;
    /// # use deku::prelude::*;
    /// # use deku::ctx::Endian;
    /// #[derive(Debug, DekuRead, DekuWrite, PartialEq, Eq, Clone)]
    /// #[deku(endian = "ctx_endian", ctx = "ctx_endian: Endian")]
    /// struct EcHdr {
    ///     magic: [u8; 4],
    ///     version: u8,
    /// }
    ///
    /// let mut file = File::options().read(true).open("file").unwrap();
    /// file.seek(SeekFrom::Start(0)).unwrap();
    /// let mut reader = Reader::new(&mut file);
    /// let ec = EcHdr::from_reader_with_ctx(&mut reader, Endian::Big).unwrap();
    /// ```
    fn from_reader_with_ctx<R: no_std_io::Read + no_std_io::Seek>(
        reader: &mut Reader<R>,
        ctx: Ctx,
    ) -> Result<Self, DekuError>
    where
        Self: Sized;
}

/// "Reader" trait: implemented on DekuRead struct and enum containers. A `container` is a type which
/// doesn't need any context information.
#[rustversion::attr(
    since(1.78),
    diagnostic::on_unimplemented(
        note = "implement by adding #[derive(DekuRead)] to `{Self}`",
        note = "make sure the `ctx` sent into the function matches `{Self}`'s `ctx`",
    )
)]
pub trait DekuContainerRead<'a>: DekuReader<'a, ()> {
    /// Construct type from Reader implementing [`no_std_io::Read`].
    /// * **input** - Input given as "Reader" and bit offset
    ///
    /// # Returns
    /// (amount of total bits read, Self)
    ///
    /// [BufRead]: std::io::BufRead
    ///
    /// # Example
    /// ```rust, no_run
    /// # use std::io::{Seek, SeekFrom, Read};
    /// # use std::fs::File;
    /// # use deku::prelude::*;
    /// #[derive(Debug, DekuRead, DekuWrite, PartialEq, Eq, Clone)]
    /// #[deku(endian = "big")]
    /// struct EcHdr {
    ///     magic: [u8; 4],
    ///     version: u8,
    /// }
    /// let mut file = File::options().read(true).open("file").unwrap();
    /// file.seek(SeekFrom::Start(0)).unwrap();
    /// let ec = EcHdr::from_reader((&mut file, 0)).unwrap();
    /// ```
    fn from_reader<R: no_std_io::Read + no_std_io::Seek>(
        input: (&'a mut R, usize),
    ) -> Result<(usize, Self), DekuError>
    where
        Self: Sized;

    /// Read bytes and construct type
    /// * **input** - Input given as data and bit offset
    ///
    /// Returns the remaining bytes and bit offset after parsing in addition to Self.
    fn from_bytes(input: (&'a [u8], usize)) -> Result<((&'a [u8], usize), Self), DekuError>
    where
        Self: Sized;
}

/// "Writer" trait: write from type to bytes
#[rustversion::attr(
    since(1.78),
    diagnostic::on_unimplemented(
        note = "implement by adding #[derive(DekuRead)] to `{Self}`",
        note = "make sure the `ctx` sent into the function matches `{Self}`'s `ctx`",
    )
)]
pub trait DekuWriter<Ctx = ()> {
    /// Write type to bytes
    fn to_writer<W: no_std_io::Write + no_std_io::Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError>;
}

/// "Writer" trait: implemented on DekuWrite struct and enum containers. A `container` is a type which
/// doesn't need any context information.
#[rustversion::attr(
    since(1.78),
    diagnostic::on_unimplemented(
        note = "implement by adding #[derive(DekuWrite)] to `{Self}`",
        note = "make sure the `ctx` sent into the function matches `{Self}`'s `ctx`",
    )
)]
pub trait DekuContainerWrite: DekuWriter<()> {
    /// Write struct/enum to Vec<u8>
    ///
    /// ```rust
    /// # use deku::prelude::*;
    /// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    /// #[deku(endian = "little")]
    /// struct S {
    ///    data_00: u8,
    ///    data_01: u16,
    ///    data_02: u32,
    /// }
    ///
    /// let s = S { data_00: 0x01, data_01: 0x02, data_02: 0x03 };
    /// let bytes = s.to_bytes().unwrap();
    /// assert_eq!(bytes, [0x01, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00]);
    /// ````
    #[inline(always)]
    fn to_bytes(&self) -> Result<Vec<u8>, DekuError> {
        let mut out_buf = Vec::new();
        let mut cursor = no_std_io::Cursor::new(&mut out_buf);
        let mut __deku_writer = Writer::new(&mut cursor);
        DekuWriter::to_writer(self, &mut __deku_writer, ())?;
        __deku_writer.finalize()?;
        Ok(out_buf)
    }

    /// Write struct/enum to a given slice
    ///
    /// ```rust
    /// # use deku::prelude::*;
    /// #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    /// #[deku(endian = "little")]
    /// struct S {
    ///    data_00: u8,
    ///    data_01: u16,
    ///    data_02: u32,
    /// }
    ///
    /// let mut buf = [0; 7];
    /// let s = S { data_00: 0x01, data_01: 0x02, data_02: 0x03 };
    /// let amt_written = s.to_slice(&mut buf).unwrap();
    /// assert_eq!(buf, [0x01, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00]);
    /// assert_eq!(amt_written, 7)
    /// ````
    #[inline(always)]
    fn to_slice(&self, buf: &mut [u8]) -> Result<usize, DekuError> {
        let mut cursor = no_std_io::Cursor::new(buf);
        let mut writer = Writer::new(&mut cursor);
        DekuWriter::to_writer(self, &mut writer, ())?;
        writer.finalize()?;

        Ok(writer.bits_written / 8)
    }

    /// Write struct/enum to BitVec
    ///
    /// ```rust
    /// # use deku::prelude::*;
    /// # use deku::bitvec::Lsb0;
    /// #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    /// #[deku(endian = "little")]
    /// pub struct TestOver {
    ///     #[deku(bits = "4")]
    ///     pub a: u8,
    ///     #[deku(bits = "4")]
    ///     pub b: u8,
    ///     #[deku(bits = "1")]
    ///     pub c: u8,
    /// }
    ///
    /// let test_data: &[u8] = &[0xf1, 0x80];
    /// let test = TestOver::from_bytes((test_data, 0)).unwrap().1;
    /// let bits = test.to_bits().unwrap();
    /// assert_eq!(deku::bitvec::bitvec![1, 1, 1, 1, 0, 0, 0, 1, 1], bits);
    /// ```
    #[inline(always)]
    #[cfg(feature = "bits")]
    fn to_bits(&self) -> Result<bitvec::BitVec<u8, bitvec::Msb0>, DekuError> {
        let mut out_buf = Vec::new();
        let mut cursor = no_std_io::Cursor::new(&mut out_buf);
        let mut __deku_writer = Writer::new(&mut cursor);
        DekuWriter::to_writer(self, &mut __deku_writer, ())?;
        let mut leftover = __deku_writer.leftover;
        let mut bv = bitvec::BitVec::from_slice(&out_buf);
        bv.append(&mut leftover.0);
        Ok(bv)
    }
}

/// "Updater" trait: apply mutations to a type
pub trait DekuUpdate {
    /// Apply updates
    fn update(&mut self) -> Result<(), DekuError>;
}

/// "Extended Enum" trait: obtain additional enum information
pub trait DekuEnumExt<'__deku, T> {
    /// Obtain `id` of a given enum variant
    fn deku_id(&self) -> Result<T, DekuError>;
}

impl<T, Ctx> DekuWriter<Ctx> for &T
where
    T: DekuWriter<Ctx>,
    Ctx: Copy,
{
    #[inline(always)]
    fn to_writer<W: no_std_io::Write + no_std_io::Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        <T>::to_writer(self, writer, ctx)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/test_common/mod.rs"]
pub mod test_common;
