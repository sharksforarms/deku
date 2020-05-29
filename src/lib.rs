//! Deku: Declaritive binary reading and writing
//!
//! This crate provides bit-level, symetric, serialization/deserialization implementations for your structs and enums
//! This allows the developer to focus on building and maintaining the representation of data and not on serialization/deserialization code.
//!
//! This approach is especially usefull when dealing with binary structures or network protocols
//!
//! Under the hood, it makes use of the [bitvec](https://crates.io/crates/bitvec) crate as the "Reader" and “Writer”
//!
//! Example
//! ```
//! use deku::prelude::*;
//!
//! #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
//! #[deku(endian = "big")]
//! struct DekuTest {
//!     #[deku(bits = "4")]
//!     field_a: u8,
//!     #[deku(bits = "4")]
//!     field_b: u8,
//!     field_c: u16,
//! }
//!
//! let data: &[u8] = [0b0110_1001, 0xBE, 0xEF].as_ref();
//! let (_rest, mut val) = DekuTest::from_bytes((data, 0)).unwrap();
//! assert_eq!(DekuTest {
//!     field_a: 0b0110,
//!     field_b: 0b1001,
//!     field_c: 0xBEEF,
//! }, val);
//!
//! val.field_c = 0xC0FE;
//!
//! let data_out = val.to_bytes();
//! assert_eq!(vec![0b0110_1001, 0xC0, 0xFE], data_out);
//! ```
use bitvec::prelude::*;
pub use deku_derive::*;
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
                // We need to do this because it could be split accross byte boundaries
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
                let res = if output_is_le {
                    self.to_le_bytes()
                } else {
                    self.to_be_bytes()
                };

                let mut res_bits: BitVec<Msb0, u8> = res.to_vec().into();

                // Truncate to fit in bit_size bits
                if let Some(max_bits) = bit_size {
                    if res_bits.len() > max_bits {
                        res_bits = res_bits.split_off(res_bits.len() - max_bits);
                    }
                }

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
        case::normal(0xDDCCBBAA, IS_LE, None, vec![0xAA, 0xBB, 0xCC, 0xDD]),
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
