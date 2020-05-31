/*!
# Deku: Declarative binary reading and writing

Deriving a struct or enum with `DekuRead` and `DekuWrite` provides bit-level, symmetric, serialization/deserialization implementations.

This allows the developer to focus on building and maintaining how the data is represented and manipulated and not on redundant, error-prone, parsing/writing code.

This approach is especially useful when dealing with binary structures such as TLVs or network protocols.

Under the hood, it makes use of the [bitvec](https://crates.io/crates/bitvec) crate as the "Reader" and “Writer”

For documentation and examples on available `#deku[()]` attributes and features, see [attributes list](attributes/index.html)

For more examples, see the [examples folder](https://github.com/sharksforarms/deku/tree/master/examples)!

# Simple Example

Let's read big-endian data into a struct, with fields containing different sizes, modify a value, and write it back

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

let data_out = val.to_bytes();
assert_eq!(vec![0b0110_1001, 0xC0, 0xFE], data_out);
```

# Composing

Deku structs/enums can be composed as long as they implement BitsReader / BitsWriter! (Which DekuRead/DekuWrite implement)

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

let data_out = val.to_bytes();
assert_eq!(data, data_out);
```

# Vec

Vec<T> can be used in combination with the [len](attributes/index.html#len) attribute (T must implement BitsReader/BitsWriter)

If the length of Vec changes, the original field specified in `len` will not get updated.
Calling `.update()` can be used to "update" the original field!

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    count: u8,
    #[deku(len = "count")]
    data: Vec<u8>,
}

let data: Vec<u8> = vec![0x02, 0xBE, 0xEF, 0xFF, 0xFF];
let (_rest, mut val) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();
assert_eq!(DekuTest {
    count: 0x02,
    data: vec![0xBE, 0xEF]
}, val);

let data_out = val.to_bytes();
assert_eq!(vec![0x02, 0xBE, 0xEF], data_out);

// Pushing an element to data
val.data.push(0xAA);

assert_eq!(DekuTest {
    count: 0x02, // Note: this value has not changed
    data: vec![0xBE, 0xEF, 0xAA]
}, val);

let data_out = val.to_bytes();
// Note: `count` is still 0x02 while 3 bytes got written
assert_eq!(vec![0x02, 0xBE, 0xEF, 0xAA], data_out);

// Use `update` to update `count`
val.update();

assert_eq!(DekuTest {
    count: 0x03,
    data: vec![0xBE, 0xEF, 0xAA]
}, val);

```

# Enums

As enums can have multiple variants, each variant must have a way to match on the incoming data.

First the "type" is read using the `id_type`, then is matched against the variants given `id`. What happens after is the same as structs!

This is implemented with the [id](/attributes/index.html#id) and [id_type](attributes/index.html#id_type) attributes.

Example:

```rust
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
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

*/
use bitvec::prelude::*;
pub use deku_derive::*;
pub mod attributes;
pub mod error;
pub mod prelude;
use crate::error::DekuError;

/// "Reader" trait: read bits and construct type
pub trait BitsReader {
    /// Read bits and construct type
    /// * **input** - Input as bits
    /// * **input_is_le** - `true` if input is to be interpreted as little endian,
    /// false otherwise (controlled via `endian` deku attribute)
    /// * **bit_size** - `Some` if `bits` or `bytes` deku attributes provided,
    /// `None` otherwise
    /// * **count** - Number of elements to read for container, Some if `len` attribute
    /// is provided, else None
    fn read(
        input: &BitSlice<Msb0, u8>,
        input_is_le: bool,
        bit_size: Option<usize>,
        count: Option<usize>,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized;
}

/// "Writer" trait: write from type to bits
pub trait BitsWriter {
    /// Write type to bits
    /// * **output_is_le** - `true` if output is to be interpreted as little endian,
    /// false otherwise (controlled via `endian` deku attribute)
    /// * **bit_size** - `Some` if `bits` or `bytes` deku attributes provided,
    /// `None` otherwise
    fn write(&self, output_is_le: bool, bit_size: Option<usize>) -> BitVec<Msb0, u8>;
}

macro_rules! ImplDekuTraits {
    ($typ:ty) => {
        impl BitsReader for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                input_is_le: bool,
                bit_size: Option<usize>,
                count: Option<usize>,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                assert!(count.is_none(), "Dev error: `count` should always be None");

                let max_type_bits: usize = std::mem::size_of::<$typ>() * 8;

                let bit_size = match bit_size {
                    None => max_type_bits,
                    Some(s) if s > max_type_bits => {
                        return Err(DekuError::Parse(format!(
                            "too much data: container of {} cannot hold {}",
                            max_type_bits, s
                        )))
                    }
                    Some(s) => s,
                };
                if input.len() < bit_size {
                    return Err(DekuError::Parse(format!(
                        "not enough data: expected {} got {}",
                        bit_size,
                        input.len()
                    )));
                }

                let (bit_slice, rest) = input.split_at(bit_size);

                // Create a new BitVec from the slice
                // We need to do this because it could be split across byte boundaries
                // i.e. BitSlice<Msb0, u8> [00, 1100].load_le() == 48
                // vs BitSlice<Msb0, u8> [001100].load_le() == 12
                let mut bits: BitVec<Msb0, u8> = BitVec::new();
                for b in bit_slice {
                    bits.push(*b);
                }

                let value = if input_is_le {
                    bits.load_le()
                } else {
                    bits.load_be()
                };

                Ok((rest, value))
            }
        }

        impl BitsWriter for $typ {
            fn write(&self, output_is_le: bool, bit_size: Option<usize>) -> BitVec<Msb0, u8> {
                let input = if output_is_le {
                    self.to_le_bytes()
                } else {
                    self.to_be_bytes()
                };

                let input_bits: BitVec<Msb0, u8> = input.to_vec().into();

                let res_bits: BitVec<Msb0, u8> = {
                    if let Some(bit_size) = bit_size {
                        if bit_size > input_bits.len() {
                            todo!() // TODO: return err
                        }

                        if output_is_le {
                            // Example read 10 bits u32 [0xAB, 0b11_000000]
                            // => [10101011, 00000011, 00000000, 00000000]
                            let mut res_bits = BitVec::<Msb0, u8>::with_capacity(bit_size);
                            let mut remaining_bits = bit_size;
                            for chunk in input_bits.chunks(8) {
                                if chunk.len() > remaining_bits {
                                    let bits = &chunk[chunk.len() - remaining_bits..];
                                    for b in bits {
                                        res_bits.push(*b);
                                    }
                                    // https://github.com/myrrlyn/bitvec/issues/62
                                    // res_bits.extend_from_slice(chunk[chunk.len() - remaining_bits..]);
                                    break;
                                } else {
                                    for b in chunk {
                                        res_bits.push(*b);
                                    }
                                    // https://github.com/myrrlyn/bitvec/issues/62
                                    // res_bits.extend_from_slice(chunk)
                                }
                                remaining_bits -= chunk.len();
                            }

                            res_bits
                        } else {
                            // Example read 10 bits u32 [0xAB, 0b11_000000]
                            // => [00000000, 00000000, 00000010, 10101111]
                            input_bits[input_bits.len() - bit_size..].into()
                        }
                    } else {
                        input_bits
                    }
                };

                res_bits
            }
        }
    };
}

impl<T: BitsReader> BitsReader for Vec<T> {
    fn read(
        input: &BitSlice<Msb0, u8>,
        input_is_le: bool,
        bit_size: Option<usize>,
        count: Option<usize>,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let count = count.expect("Dev error: `count` should always be Some");

        let mut res = Vec::with_capacity(count);
        let mut rest = input;
        for _i in 0..count {
            let (new_rest, val) = <T>::read(rest, input_is_le, bit_size, None)?;
            res.push(val);
            rest = new_rest;
        }

        Ok((rest, res))
    }
}

impl<T: BitsWriter> BitsWriter for Vec<T> {
    fn write(&self, output_is_le: bool, bit_size: Option<usize>) -> BitVec<Msb0, u8> {
        let mut acc = BitVec::new();

        for v in self {
            let r = v.write(output_is_le, bit_size);
            acc.extend(r);
        }

        acc
    }
}

ImplDekuTraits!(u8);
ImplDekuTraits!(u16);
ImplDekuTraits!(u32);
ImplDekuTraits!(u64);
// ImplDekuTraits!(u128);
ImplDekuTraits!(usize);

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[cfg(target_endian = "little")]
    static IS_LE: bool = true;

    #[cfg(target_endian = "big")]
    static IS_LE: bool = false;

    #[rstest(input,input_is_le,bit_size,count,expected,expected_rest,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), IS_LE, Some(32), None, 0xAABBCCDD, bits![Msb0, u8;]),
        case::normal_offset([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), IS_LE, Some(12), None, 0b1110_1001_0110, bits![Msb0, u8; 0,0,0,0, 1,1,0,0,1,1,0,0, 1,1,0,1,1,1,0,1]),

        // TODO: Better error message for these
        #[should_panic(expected="Parse(\"not enough data: expected 32 got 0\")")]
        case::not_enough_data([].as_ref(), IS_LE, Some(32), None, 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"not enough data: expected 32 got 16\")")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), IS_LE, Some(32), None, 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"too much data: container of 32 cannot hold 64\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), IS_LE, Some(64), None, 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected="Dev error: `count` should always be None")]
        case::dev_err_count_some([].as_ref(), IS_LE, Some(64), Some(1), 0xFF, bits![Msb0, u8;]),
    )]
    fn test_bit_read(
        input: &[u8],
        input_is_le: bool,
        bit_size: Option<usize>,
        count: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = u32::read(bit_slice, input_is_le, bit_size, count).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input,output_is_le,bit_size,expected,
        case::normal_le(0xDDCCBBAA, IS_LE, None, vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be(0xDDCCBBAA, !IS_LE, None, vec![0xDD, 0xCC, 0xBB, 0xAA]),
        case::bit_size_le_smaller(0x03AB, IS_LE, Some(10), vec![0xAB, 0b11_000000]),
        case::bit_size_be_smaller(0x03AB, !IS_LE, Some(10), vec![0b11, 0xAB]),
        #[should_panic(expected = "not yet implemented")] // TODO:
        case::bit_size_le_bigger(0x03AB, IS_LE, Some(100), vec![0xAB, 0b11_000000]),
    )]
    fn test_bit_write(input: u32, output_is_le: bool, bit_size: Option<usize>, expected: Vec<u8>) {
        let res_write = input.write(output_is_le, bit_size).into_vec();
        assert_eq!(expected, res_write);
    }

    #[rstest(input,is_le,bit_size,expected,expected_rest,expected_write,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), IS_LE, Some(32), 0xAABBCCDD, bits![Msb0, u8;], vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        is_le: bool,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = u32::read(bit_slice, is_le, bit_size, None).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let res_write = res_read.write(is_le, bit_size).into_vec();
        assert_eq!(expected_write, res_write);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }

    #[rstest(input,input_is_le,bit_size,count,expected,expected_rest,
        case::count_0([0xAA].as_ref(), IS_LE, Some(8), Some(0), vec![], bits![Msb0, u8; 1,0,1,0,1,0,1,0]),
        case::count_1([0xAA, 0xBB].as_ref(), IS_LE, Some(8), Some(1), vec![0xAA], bits![Msb0, u8; 1,0,1,1,1,0,1,1]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), IS_LE, Some(8), Some(2), vec![0xAA, 0xBB], bits![Msb0, u8; 1,1,0,0,1,1,0,0]),

        case::bits_6([0b0110_1001, 0b1110_1001].as_ref(), IS_LE, Some(6), Some(2), vec![0b00_011010, 0b00_011110], bits![Msb0, u8; 1,0,0,1]),

        #[should_panic(expected="Parse(\"too much data: container of 8 cannot hold 9\")")]
        case::not_enough_data([].as_ref(), IS_LE, Some(9), Some(1), vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"too much data: container of 8 cannot hold 9\")")]
        case::not_enough_data([0xAA].as_ref(), IS_LE, Some(9), Some(1), vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"not enough data: expected 8 got 0\")")]
        case::not_enough_data([0xAA].as_ref(), IS_LE, Some(8), Some(2), vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"too much data: container of 8 cannot hold 9\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), IS_LE, Some(9), Some(1), vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Dev error: `count` should always be Some")]
        case::dev_err_count_none([].as_ref(), IS_LE, Some(0), None, vec![], bits![Msb0, u8;]),
    )]
    fn test_vec_read(
        input: &[u8],
        input_is_le: bool,
        bit_size: Option<usize>,
        count: Option<usize>,
        expected: Vec<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = Vec::<u8>::read(bit_slice, input_is_le, bit_size, count).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input,output_is_le,bit_size,expected,
        case::normal(vec![0xAABB, 0xCCDD], IS_LE, None, vec![0xBB, 0xAA, 0xDD, 0xCC]),
    )]
    fn test_vec_write(
        input: Vec<u16>,
        output_is_le: bool,
        bit_size: Option<usize>,
        expected: Vec<u8>,
    ) {
        let res_write = input.write(output_is_le, bit_size).into_vec();
        assert_eq!(expected, res_write);
    }

    #[rstest(input,input_is_le,bit_size,count,expected,expected_rest,expected_write,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), IS_LE, Some(8), Some(4), vec![0xAA, 0xBB, 0xCC, 0xDD], bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_vec_read_write(
        input: &[u8],
        input_is_le: bool,
        bit_size: Option<usize>,
        count: Option<usize>,
        expected: Vec<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = Vec::<u8>::read(bit_slice, input_is_le, bit_size, count).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let res_write: Vec<u8> = res_read.into();
        assert_eq!(expected_write, res_write);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }
}
