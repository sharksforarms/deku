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

For documentation and examples on available `#deku[()]` attributes and features,
see [attributes list](attributes/index.html)

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

Vec<T> can be used in combination with the [count](attributes/index.html#count)
attribute (T must implement DekuRead/DekuWrite)

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

First the "type" is read using the `type`, then is matched against the
variants given `id`. What happens after is the same as structs!

This is implemented with the [id](/attributes/index.html#id) and
[type](attributes/index.html#type) attributes.

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

let data: Vec<u8> = vec![0x01, 0x02, 0xEF, 0xBE];

let (rest, val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
assert_eq!(DekuTest::VariantA , val);

let (rest, val) = DekuTest::from_bytes(rest).unwrap();
assert_eq!(DekuTest::VariantB(0xBEEF) , val);
```

# Context

Child parsers can get access to the parent's parsed values using the `ctx` attribute

For more information see [ctx attribute](attributes/index.html#ctx)

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

let data: Vec<u8> = vec![0x01, 0x02];

let (rest, value) = Root::from_bytes((&data[..], 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02)
```

*/
#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{format, vec::Vec};

#[cfg(feature = "std")]
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::ctx::{BitSize, Count, Endian};
use bitvec::prelude::*;
use core::convert::TryInto;
pub use deku_derive::*;

pub mod attributes;
pub mod ctx;
pub mod error;
pub mod prelude;
mod slice_impls;

use crate::error::DekuError;

/// "Reader" trait: read bits and construct type
pub trait DekuRead<Ctx = ()> {
    /// Read bits and construct type
    /// * **input** - Input as bits
    /// * **ctx** - A context required by context-sensitive reading. A unit type `()` means no context
    /// needed.
    fn read(input: &BitSlice<Msb0, u8>, ctx: Ctx) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized;
}

/// "Reader" trait: implemented on DekuRead struct and enum containers. A `container` is a type which
/// doesn't need any context information.
pub trait DekuContainerRead: DekuRead<()> {
    /// Read bytes and construct type
    /// * **input** - Input as a tuple of (bytes, bit_offset)
    ///
    /// Returns a tuple of the remaining data as (bytes, bit_offset) and a constructed value
    fn from_bytes(input: (&[u8], usize)) -> Result<((&[u8], usize), Self), DekuError>
    where
        Self: Sized;
}

/// "Writer" trait: write from type to bits
pub trait DekuWrite<Ctx = ()> {
    /// Write type to bits
    /// * **output** - Sink to store resulting bits
    /// * **ctx** - A context required by context-sensitive reading. A unit type `()` means no context
    /// needed.
    fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError>;
}

/// "Writer" trait: implemented on DekuWrite struct and enum containers. A `container` is a type which
/// doesn't need any context information.
pub trait DekuContainerWrite: DekuWrite<()> {
    /// Write struct/enum to Vec<u8>
    fn to_bytes(&self) -> Result<Vec<u8>, DekuError>;

    /// Write struct/enum to BitVec
    fn to_bits(&self) -> Result<BitVec<Msb0, u8>, DekuError>;
}

/// "Updater" trait: apply mutations to a type
pub trait DekuUpdate {
    /// Apply updates
    fn update(&mut self) -> Result<(), DekuError>;
}

macro_rules! ImplDekuTraits {
    ($typ:ty) => {
        impl DekuRead<(Endian, BitSize)> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                (endian, bit_size): (Endian, BitSize),
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let max_type_bits: usize = BitSize::of::<$typ>().into();
                let bit_size: usize = bit_size.into();

                let input_is_le = endian.is_le();

                if bit_size > max_type_bits {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {} bits cannot hold {} bits",
                        max_type_bits, bit_size
                    )));
                }

                if input.len() < bit_size {
                    return Err(DekuError::Parse(format!(
                        "not enough data: expected {} bits got {} bits",
                        bit_size,
                        input.len()
                    )));
                }

                let (bit_slice, rest) = input.split_at(bit_size);

                let pad = 8 * ((bit_slice.len() + 7) / 8) - bit_slice.len();

                let value = if pad == 0 && bit_slice.len() == max_type_bits {
                    // if everything is aligned, just read the value

                    let bytes: &[u8] = bit_slice.as_slice();

                    // Read value
                    if input_is_le {
                        <$typ>::from_le_bytes(bytes.try_into()?)
                    } else {
                        <$typ>::from_be_bytes(bytes.try_into()?)
                    }
                } else {
                    // Create a new BitVec from the slice and pad un-aligned chunks
                    // i.e. [10010110, 1110] -> [10010110, 00001110]
                    let bits: BitVec<Msb0, u8> = {
                        let mut bits = BitVec::with_capacity(bit_slice.len() + pad);

                        // Copy bits to new BitVec
                        for b in bit_slice {
                            bits.push(*b);
                        }

                        // Force align
                        //i.e. [1110, 10010110] -> [11101001, 0110]
                        bits.force_align();

                        // Some padding to next byte
                        if input_is_le {
                            let ins_index = bits.len() - (8 - pad);
                            for _ in 0..pad {
                                bits.insert(ins_index, false);
                            }
                        } else {
                            for _ in 0..pad {
                                bits.insert(0, false);
                            }
                        }

                        // Pad up-to size of type
                        for _ in 0..(max_type_bits - bits.len()) {
                            if input_is_le {
                                bits.push(false);
                            } else {
                                bits.insert(0, false);
                            }
                        }

                        bits
                    };

                    let bytes: &[u8] = bits.as_slice();

                    // Read value
                    if input_is_le {
                        <$typ>::from_le_bytes(bytes.try_into()?)
                    } else {
                        <$typ>::from_be_bytes(bytes.try_into()?)
                    }
                };

                Ok((rest, value))
            }
        }

        // Only have `endian`, set `bit_size` to `BitSize::of::<Type>()`
        impl DekuRead<Endian> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                endian: Endian,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let max_type_bits = BitSize::of::<$typ>();

                <$typ>::read(input, (endian, max_type_bits))
            }
        }

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuRead<BitSize> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                bit_size: BitSize,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let endian = Endian::default();

                <$typ>::read(input, (endian, bit_size))
            }
        }

        impl DekuRead for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                _: (),
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                <$typ>::read(input, Endian::default())
            }
        }

        impl DekuWrite<(Endian, BitSize)> for $typ {
            fn write(
                &self,
                output: &mut BitVec<Msb0, u8>,
                (endian, bit_size): (Endian, BitSize),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                let bit_size: usize = bit_size.into();

                let input_bits = input.view_bits::<Msb0>();

                if bit_size > input_bits.len() {
                    return Err(DekuError::InvalidParam(format!(
                        "bit size {} is larger then input {}",
                        bit_size,
                        input_bits.len()
                    )));
                }

                if matches!(endian, Endian::Little) {
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [10101011, 00000011, 00000000, 00000000]
                    let mut remaining_bits = bit_size;
                    for chunk in input_bits.chunks(8) {
                        if chunk.len() > remaining_bits {
                            let bits = &chunk[chunk.len() - remaining_bits..];
                            for b in bits {
                                output.push(*b);
                            }
                            // https://github.com/myrrlyn/bitvec/issues/62
                            // output.extend_from_slice(chunk[chunk.len() - remaining_bits..]);
                            break;
                        } else {
                            for b in chunk {
                                output.push(*b);
                            }
                            // https://github.com/myrrlyn/bitvec/issues/62
                            // output.extend_from_slice(chunk)
                        }
                        remaining_bits -= chunk.len();
                    }
                } else {
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [00000000, 00000000, 00000010, 10101111]
                    let bits = &input_bits[input_bits.len() - bit_size..];
                    for b in bits {
                        output.push(*b);
                    }
                    // https://github.com/myrrlyn/bitvec/issues/62
                    // output.extend_from_slice(input_bits[input_bits.len() - bit_size..]);
                }
                Ok(())
            }
        }

        // Only have `endian`, return all input
        impl DekuWrite<Endian> for $typ {
            fn write(
                &self,
                output: &mut BitVec<Msb0, u8>,
                endian: Endian,
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };
                output.extend_from_bitslice(input.view_bits());
                Ok(())
            }
        }

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuWrite<BitSize> for $typ {
            fn write(
                &self,
                output: &mut BitVec<Msb0, u8>,
                bit_size: BitSize,
            ) -> Result<(), DekuError> {
                <$typ>::write(self, output, (Endian::default(), bit_size))
            }
        }

        impl DekuWrite for $typ {
            fn write(&self, output: &mut BitVec<Msb0, u8>, _: ()) -> Result<(), DekuError> {
                <$typ>::write(self, output, Endian::default())
            }
        }
    };
}

impl<T: DekuRead<Ctx>, Ctx: Copy> DekuRead<(Count, Ctx)> for Vec<T> {
    /// Read the specified number of `T`s from input.
    /// * `count` - the number of `T`s you want to read.
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use bitvec::view::BitView;
    /// let input = vec![1u8, 2, 3, 4];
    /// let (rest, v) = Vec::<u32>::read(input.view_bits(), (1.into(), Endian::Little)).unwrap();
    /// assert!(rest.is_empty());
    /// assert_eq!(v, vec![0x04030201])
    /// ```
    fn read(
        input: &BitSlice<Msb0, u8>,
        (count, inner_ctx): (Count, Ctx),
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let count: usize = count.into();

        let mut res = Vec::with_capacity(count);
        let mut rest = input;
        for _i in 0..count {
            let (new_rest, val) = <T>::read(rest, inner_ctx)?;
            res.push(val);
            rest = new_rest;
        }

        Ok((rest, res))
    }
}

impl<T: DekuRead> DekuRead<Count> for Vec<T> {
    /// Read the specified number of `T`s from input for types which don't require context.
    fn read(
        input: &BitSlice<Msb0, u8>,
        count: Count,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        Vec::read(input, (count, ()))
    }
}

impl<T: DekuWrite<Ctx>, Ctx: Copy> DekuWrite<Ctx> for Vec<T> {
    /// Write all `T`s in a `Vec` to bits.
    /// * **inner_ctx** - The context required by `T`.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite, prelude::{Lsb0, Msb0}};
    /// # use bitvec::bitvec;
    /// let data = vec![1u8];
    /// let mut output = bitvec![Msb0, u8;];
    /// data.write(&mut output, Endian::Big).unwrap();
    /// assert_eq!(output, bitvec![0, 0, 0, 0, 0, 0, 0, 1])
    /// ```
    fn write(&self, output: &mut BitVec<Msb0, u8>, inner_ctx: Ctx) -> Result<(), DekuError> {
        for v in self {
            v.write(output, inner_ctx)?;
        }
        Ok(())
    }
}

impl<T: DekuRead<Ctx>, Ctx: Copy> DekuRead<Ctx> for Option<T> {
    /// Read a T from input and store as Some(T)
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use bitvec::view::BitView;
    /// let input = vec![1u8, 2, 3, 4];
    /// let (rest, v) = Option::<u32>::read(input.view_bits(), Endian::Little).unwrap();
    /// assert!(rest.is_empty());
    /// assert_eq!(v, Some(0x04030201))
    /// ```
    fn read(
        input: &BitSlice<Msb0, u8>,
        inner_ctx: Ctx,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, val) = <T>::read(input, inner_ctx)?;
        Ok((rest, Some(val)))
    }
}

impl<T: DekuWrite<Ctx>, Ctx: Copy> DekuWrite<Ctx> for Option<T> {
    /// Write T if Some
    /// * **inner_ctx** - The context required by `T`.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite, prelude::{Lsb0, Msb0}};
    /// # use bitvec::bitvec;
    /// let data = Some(1u8);
    /// let mut output = bitvec![Msb0, u8;];
    /// data.write(&mut output, Endian::Big).unwrap();
    /// assert_eq!(output, bitvec![0, 0, 0, 0, 0, 0, 0, 1])
    /// ```
    fn write(&self, output: &mut BitVec<Msb0, u8>, inner_ctx: Ctx) -> Result<(), DekuError> {
        if let Some(v) = self {
            v.write(output, inner_ctx)
        } else {
            Ok(())
        }
    }
}

ImplDekuTraits!(u8);
ImplDekuTraits!(u16);
ImplDekuTraits!(u32);
ImplDekuTraits!(u64);
ImplDekuTraits!(u128);
ImplDekuTraits!(usize);
ImplDekuTraits!(i8);
ImplDekuTraits!(i16);
ImplDekuTraits!(i32);
ImplDekuTraits!(i64);
ImplDekuTraits!(i128);
ImplDekuTraits!(isize);
ImplDekuTraits!(f32);
ImplDekuTraits!(f64);

#[cfg(feature = "std")]
impl<Ctx> DekuRead<Ctx> for Ipv4Addr
where
    u32: DekuRead<Ctx>,
{
    fn read(input: &BitSlice<Msb0, u8>, ctx: Ctx) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, ip) = u32::read(input, ctx)?;
        Ok((rest, ip.into()))
    }
}

#[cfg(feature = "std")]
impl<Ctx> DekuWrite<Ctx> for Ipv4Addr
where
    u32: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
        let ip: u32 = (*self).into();
        ip.write(output, ctx)
    }
}

#[cfg(feature = "std")]
impl<Ctx> DekuRead<Ctx> for Ipv6Addr
where
    u128: DekuRead<Ctx>,
{
    fn read(input: &BitSlice<Msb0, u8>, ctx: Ctx) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, ip) = u128::read(input, ctx)?;
        Ok((rest, ip.into()))
    }
}

#[cfg(feature = "std")]
impl<Ctx> DekuWrite<Ctx> for Ipv6Addr
where
    u128: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
        let ip: u128 = (*self).into();
        ip.write(output, ctx)
    }
}

#[cfg(feature = "std")]
impl<Ctx> DekuWrite<Ctx> for IpAddr
where
    Ipv6Addr: DekuWrite<Ctx>,
    Ipv4Addr: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
        match self {
            IpAddr::V4(ipv4) => ipv4.write(output, ctx),
            IpAddr::V6(ipv6) => ipv6.write(output, ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    static ENDIAN: Endian = Endian::new();

    macro_rules! TestPrimitive {
        ($test_name:ident, $typ:ty, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let input = $input;
                let bit_slice = input.view_bits::<Msb0>();
                let (_rest, res_read) = <$typ>::read(bit_slice, ENDIAN).unwrap();
                assert_eq!($expected, res_read);

                let mut res_write = bitvec![Msb0, u8;];
                res_read.write(&mut res_write, ENDIAN).unwrap();
                assert_eq!(input, res_write.into_vec());
            }
        };
    }

    TestPrimitive!(test_u8, u8, vec![0xAAu8], 0xAAu8);
    TestPrimitive!(test_u16, u16, vec![0xABu8, 0xCD], 0xCDAB);
    TestPrimitive!(test_u32, u32, vec![0xABu8, 0xCD, 0xEF, 0xBE], 0xBEEFCDAB);
    TestPrimitive!(
        test_u64,
        u64,
        vec![0xABu8, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD, 0xFE, 0xC0],
        0xC0FECDABBEEFCDAB
    );
    TestPrimitive!(
        test_u128,
        u128,
        vec![
            0xABu8, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD, 0xFE, 0xC0, 0xAB, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD,
            0xFE, 0xC0
        ],
        0xC0FECDABBEEFCDABC0FECDABBEEFCDAB
    );
    TestPrimitive!(
        test_usize,
        usize,
        vec![0xABu8, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD, 0xFE, 0xC0],
        if core::mem::size_of::<usize>() == 8 {
            0xC0FECDABBEEFCDAB
        } else {
            0xBEEFCDAB
        }
    );
    TestPrimitive!(test_i8, i8, vec![0xFBu8], -5);
    TestPrimitive!(test_i16, i16, vec![0xFDu8, 0xFE], -259);
    TestPrimitive!(test_i32, i32, vec![0x02u8, 0x3F, 0x01, 0xEF], -0x10FEC0FE);
    TestPrimitive!(
        test_i64,
        i64,
        vec![0x02u8, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF],
        -0x10FEC0FE10FEC0FE
    );
    TestPrimitive!(
        test_i128,
        i128,
        vec![
            0x02u8, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF, 0x01, 0x3F,
            0x01, 0xEF
        ],
        -0x10FEC0FE10FEC0FE10FEC0FE10FEC0FE
    );
    TestPrimitive!(
        test_isize,
        isize,
        vec![0x02u8, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF],
        if core::mem::size_of::<isize>() == 8 {
            -0x10FEC0FE10FEC0FE
        } else {
            -0x10FEC0FE
        }
    );
    TestPrimitive!(test_f32, f32, vec![0xA6u8, 0x9B, 0xC4, 0xBB], -0.006);
    TestPrimitive!(
        test_f64,
        f64,
        vec![0xFAu8, 0x7E, 0x6A, 0xBC, 0x74, 0x93, 0x78, 0xBF],
        -0.006
    );

    #[rstest(input, endian, bit_size, expected, expected_rest,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![Msb0, u8;]),
        case::normal_bits_12_le([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Little, Some(12), 0b1110_1001_0110, bits![Msb0, u8; 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1]),
        case::normal_bits_12_be([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Big, Some(12), 0b1001_0110_1110, bits![Msb0, u8; 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1]),
        case::normal_bit_6([0b1001_0110].as_ref(), Endian::Little, Some(6), 0b1001_01, bits![Msb0, u8; 1, 0,]),
        #[should_panic(expected = "Parse(\"not enough data: expected 32 bits got 0 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(32), 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"not enough data: expected 32 bits got 16 bits\")")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(32), 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 32 bits cannot hold 64 bits\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(64), 0xFF, bits![Msb0, u8;]),
    )]
    fn test_bit_read(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => u32::read(bit_slice, (endian, BitSize(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };

        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input, endian, bit_size, expected,
        case::normal_le(0xDDCC_BBAA, Endian::Little, None, vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be(0xDDCC_BBAA, Endian::Big, None, vec![0xDD, 0xCC, 0xBB, 0xAA]),
        case::bit_size_le_smaller(0x03AB, Endian::Little, Some(10), vec![0xAB, 0b11_000000]),
        case::bit_size_be_smaller(0x03AB, Endian::Big, Some(10), vec![0b11_1010_10, 0b11_000000]),
        #[should_panic(expected = "InvalidParam(\"bit size 100 is larger then input 32\")")]
        case::bit_size_le_bigger(0x03AB, Endian::Little, Some(100), vec![0xAB, 0b11_000000]),
    )]
    fn test_bit_write(input: u32, endian: Endian, bit_size: Option<usize>, expected: Vec<u8>) {
        let mut res_write = bitvec![Msb0, u8;];
        match bit_size {
            Some(bit_size) => input
                .write(&mut res_write, (endian, BitSize(bit_size)))
                .unwrap(),
            None => input.write(&mut res_write, endian).unwrap(),
        };
        assert_eq!(expected, res_write.into_vec());
    }

    #[rstest(input, endian, bit_size, expected, expected_rest, expected_write,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![Msb0, u8;], vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => u32::read(bit_slice, (endian, BitSize(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        match bit_size {
            Some(bit_size) => res_read
                .write(&mut res_write, (endian, BitSize(bit_size)))
                .unwrap(),
            None => res_read.write(&mut res_write, endian).unwrap(),
        };

        assert_eq!(expected_write, res_write.into_vec());
    }

    #[rstest(input,endian,bit_size,count,expected,expected_rest,
        case::count_0([0xAA].as_ref(), Endian::Little, Some(8), 0, vec![], bits![Msb0, u8; 1, 0, 1, 0, 1, 0, 1, 0]),
        case::count_1([0xAA, 0xBB].as_ref(), Endian::Little, Some(8), 1, vec![0xAA], bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), Endian::Little, Some(8), 2, vec![0xAA, 0xBB], bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0]),
        case::bits_6([0b0110_1001, 0b1110_1001].as_ref(), Endian::Little, Some(6), 2, vec![0b00_011010, 0b00_011110], bits![Msb0, u8; 1, 0, 0, 1]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(9), 1, vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(9), 1, vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"not enough data: expected 8 bits got 0 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(8), 2, vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(9), 1, vec![], bits![Msb0, u8;]),
    )]
    fn test_vec_read(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        count: usize,
        expected: Vec<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => {
                Vec::<u8>::read(bit_slice, (Count(count), (endian, BitSize(bit_size)))).unwrap()
            }
            None => Vec::<u8>::read(bit_slice, (Count(count), (endian))).unwrap(),
        };

        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input, endian, expected,
        case::normal(vec![0xAABB, 0xCCDD], Endian::Little, vec![0xBB, 0xAA, 0xDD, 0xCC]),
    )]
    fn test_vec_write(input: Vec<u16>, endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![Msb0, u8;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }

    #[rstest(input, endian, bit_size, count, expected, expected_rest, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2, vec![0xBBAA, 0xDDCC], bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2, vec![0xAABB, 0xCCDD], bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_vec_read_write(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        count: usize,
        expected: Vec<u16>,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let (rest, res_read) =
            Vec::<u16>::read(bit_slice, (Count(count), (endian, BitSize(bit_size)))).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read
            .write(&mut res_write, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, res_write.into_vec());

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }

    #[rstest(input, endian, expected, expected_rest,
        case::normal_le([237, 160, 254, 145].as_ref(), Endian::Little, Ipv4Addr::new(145, 254, 160, 237), bits![Msb0, u8;]),
        case::normal_be([145, 254, 160, 237].as_ref(), Endian::Big, Ipv4Addr::new(145, 254, 160, 237), bits![Msb0, u8;]),
    )]
    fn test_ipv4(
        input: &[u8],
        endian: Endian,
        expected: Ipv4Addr,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = Ipv4Addr::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, endian).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[rstest(input, endian, expected, expected_rest,
        case::normal_le([0xFF, 0x02, 0x0A, 0xC0, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].as_ref(), Endian::Little, Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff), bits![Msb0, u8;]),
        case::normal_be([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xC0, 0x0A, 0x02, 0xFF].as_ref(), Endian::Big, Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff), bits![Msb0, u8;]),
    )]
    fn test_ipv6(
        input: &[u8],
        endian: Endian,
        expected: Ipv6Addr,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = Ipv6Addr::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, endian).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[test]
    fn test_ip_addr_write() {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(145, 254, 160, 237));
        let mut ret_write = bitvec![Msb0, u8;];
        ip_addr.write(&mut ret_write, Endian::Little).unwrap();
        assert_eq!(vec![237, 160, 254, 145], ret_write.into_vec());

        let ip_addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x02ff));
        let mut ret_write = bitvec![Msb0, u8;];
        ip_addr.write(&mut ret_write, Endian::Little).unwrap();
        assert_eq!(
            vec![
                0xFF, 0x02, 0x0A, 0xC0, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00
            ],
            ret_write.into_vec()
        );
    }
}
