//! Deku is a data-to-struct serialization/deserialization library supporting bit level granularity,
//! Makes use of the [bitvec](https://crates.io/crates/bitvec) crate as the "Reader" and “Writer”

use bitvec::prelude::*;
pub use deku_derive::*;
pub mod error;
pub mod prelude;
use crate::error::DekuError;

pub trait BitsSize {
    fn bit_size() -> usize;
}

pub trait BitsReader: BitsSize {
    fn read(
        input: &BitSlice<Msb0, u8>,
        bit_size: usize,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized;
}

pub trait BitsReaderItems {
    fn read(
        input: &BitSlice<Msb0, u8>,
        bit_size: usize,
        count: usize,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized;
}

pub trait BitsWriter: BitsSize {
    fn write(self) -> Vec<u8>;
    // TODO: swap_endian Should probably be another trait because the reader also uses this
    fn swap_endian(self) -> Self;
}

macro_rules! ImplDekuTraits {
    ($typ:ty) => {
        impl BitsSize for $typ {
            fn bit_size() -> usize {
                std::mem::size_of::<$typ>() * 8
            }
        }

        impl BitsReader for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                bit_size: usize,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                if input.len() < bit_size {
                    return Err(DekuError::Parse(format!(
                        "not enough data: expected {} got {}",
                        bit_size,
                        input.len()
                    )));
                }

                if bit_size > <$typ>::bit_size() {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {} cannot hold {}",
                        <$typ>::bit_size(),
                        bit_size
                    )));
                }

                let (bits, rest) = input.split_at(bit_size);

                #[cfg(target_endian = "little")]
                let value: $typ = bits.load_be();

                #[cfg(target_endian = "big")]
                let value: $typ = bits.load_le();

                Ok((rest, value))
            }
        }

        impl BitsReaderItems for Vec<$typ> {
            fn read(
                input: &BitSlice<Msb0, u8>,
                bit_size: usize,
                count: usize,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
            where
                Self: Sized,
            {
                let expected_bits_total = bit_size * count;
                if input.len() < bit_size * count {
                    return Err(DekuError::Parse(format!(
                        "not enough data for Vec<{}>: expected {}*{}={} got {}",
                        stringify!($typ),
                        count,
                        bit_size,
                        expected_bits_total,
                        input.len()
                    )));
                }

                let mut res = Vec::with_capacity(count);
                let mut rest = input;
                for _i in 0..count {
                    let (new_rest, val) = <$typ>::read(rest, bit_size)?;
                    res.push(val);
                    rest = new_rest;
                }

                Ok((rest, res))
            }
        }

        impl BitsWriter for Vec<$typ> {
            fn write(self) -> Vec<u8> {
                let mut acc = vec![];

                for v in self {
                    let r = v.write();
                    acc.extend(r);
                }

                acc
            }

            fn swap_endian(self) -> Self {
                // TODO: should flip endian for each item?
                self
            }
        }

        impl BitsSize for Vec<$typ> {
            fn bit_size() -> usize {
                <$typ>::bit_size()
            }
        }

        impl BitsWriter for $typ {
            fn write(self) -> Vec<u8> {
                #[cfg(target_endian = "little")]
                let res = self.to_be_bytes();

                #[cfg(target_endian = "big")]
                let res = self.to_le_bytes();

                res.to_vec()
            }

            fn swap_endian(self) -> $typ {
                self.swap_bytes()
            }
        }
    };
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

    #[test]
    fn test_bit_size() {
        assert_eq!(8, u8::bit_size());
        // assert_eq!(128, u128::bit_size());
    }

    #[rstest(input,bit_size,expected,expected_rest,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 32, 0xAABBCCDD, bits![Msb0, u8;]),
        case::normal_offset([0b1001_0110, 0b1110_0000, 0xCC, 0xDD].as_ref(), 12, 0b1001_0110_1110, bits![Msb0, u8; 0,0,0,0, 1,1,0,0,1,1,0,0, 1,1,0,1,1,1,0,1]),
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF].as_ref(), 32, 0xAABBCCDD, bits![Msb0, u8; 1,1,1,0,1,1,1,0, 1,1,1,1,1,1,1,1]),

        // TODO: Better error message for these
        #[should_panic(expected="Parse(\"not enough data: expected 32 got 0\")")]
        case::not_enough_data([].as_ref(), 32, 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"not enough data: expected 32 got 16\")")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), 32, 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"too much data: container of 32 cannot hold 64\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 64, 0xFF, bits![Msb0, u8;]),
    )]
    fn test_bit_read(
        input: &[u8],
        bit_size: usize,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = u32::read(bit_slice, bit_size).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input,expected,
        case::normal(0xAABBCCDD, vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_bit_write(input: u32, expected: Vec<u8>) {
        let res_write = input.write();
        assert_eq!(expected, res_write);
    }

    #[rstest(input,bit_size,expected,expected_rest,expected_write,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 32, 0xAABBCCDD, bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        bit_size: usize,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = u32::read(bit_slice, bit_size).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let res_write = res_read.write();
        assert_eq!(expected_write, res_write);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }

    #[test]
    fn test_swap_endian() {
        let input = 0xAABBCCDDu32;
        assert_eq!(0xDDCCBBAA, input.swap_endian());
    }

    #[rstest(input,bit_size,count,expected,expected_rest,
        case::count_0([0xAA].as_ref(), 8, 0, vec![], bits![Msb0, u8; 1,0,1,0,1,0,1,0]),
        case::count_1([0xAA, 0xBB].as_ref(), 8, 1, vec![0xAA], bits![Msb0, u8; 1,0,1,1,1,0,1,1]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), 8, 2, vec![0xAA, 0xBB], bits![Msb0, u8; 1,1,0,0,1,1,0,0]),

        case::bits_6([0b0110_1001, 0b0110_1001].as_ref(), 6, 2, vec![0b00_011010, 0b00_010110], bits![Msb0, u8; 1,0,0,1]),

        #[should_panic(expected="Parse(\"not enough data for Vec<u8>: expected 1*9=9 got 0\")")]
        case::not_enough_data([].as_ref(), 9, 1, vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"not enough data for Vec<u8>: expected 1*9=9 got 8\")")]
        case::not_enough_data([0xAA].as_ref(), 9, 1, vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"not enough data for Vec<u8>: expected 2*8=16 got 8\")")]
        case::not_enough_data([0xAA].as_ref(), 8, 2, vec![], bits![Msb0, u8;]),
        #[should_panic(expected="Parse(\"too much data: container of 8 cannot hold 9\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), 9, 1, vec![], bits![Msb0, u8;]),
    )]
    fn test_vec_read(
        input: &[u8],
        bit_size: usize,
        count: usize,
        expected: Vec<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = Vec::<u8>::read(bit_slice, bit_size, count).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input,expected,
        case::normal(vec![0xAABB, 0xCCDD], vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_vec_write(input: Vec<u16>, expected: Vec<u8>) {
        let res_write = input.write();
        assert_eq!(expected, res_write);
    }

    #[rstest(input,bit_size,count,expected,expected_rest,expected_write,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 8, 4, vec![0xAA, 0xBB, 0xCC, 0xDD], bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_vec_read_write(
        input: &[u8],
        bit_size: usize,
        count: usize,
        expected: Vec<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.bits::<Msb0>();

        let (rest, res_read) = Vec::<u8>::read(bit_slice, bit_size, count).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let res_write = res_read.write();
        assert_eq!(expected_write, res_write);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }
}
