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
        bit_index: &mut usize,
        bits: usize,
    ) -> Result<Self, DekuError>
    where
        Self: Sized;
}

pub trait BitsWriter: BitsSize {
    fn write(self) -> Vec<u8>;
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
                bit_index: &mut usize,
                bits: usize,
            ) -> Result<Self, DekuError> {
                let lower_idx = *bit_index;
                let upper_idx = *bit_index + bits;
                if upper_idx > input.len() || lower_idx > input.len() {
                    return Err(DekuError::Parse(format!("Not enough data")));
                }
                let read_bits = &input[lower_idx..upper_idx];
                *bit_index += bits; // TODO: overflow?

                if read_bits.len() > <$typ>::bit_size() {
                    return Err(DekuError::Parse(format!(
                        "Parsed bits cannot fit container"
                    )));
                }

                #[cfg(target_endian = "little")]
                let res: $typ = read_bits.load_be();

                #[cfg(target_endian = "big")]
                let res: $typ = read_bits.load_le();

                Ok(res)
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

    #[rstest(input,read_bits,expected,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 32, 0xAABBCCDD),
        case::normal_offset([0b1001_0110, 0b1110_0000, 0xCC, 0xDD].as_ref(), 12, 0b1001_0110_1110),
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF].as_ref(), 32, 0xAABBCCDD),

        // TODO: Better error message for these
        #[should_panic(expected="Parse(\"Not enough data\")")]
        case::not_enough_data([].as_ref(), 32, 0xFF),
        #[should_panic(expected="Parse(\"Not enough data\")")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), 32, 0xFF),
        #[should_panic(expected="Parse(\"Parsed bits cannot fit container\")")]
        case::requesting_more_then_size([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0xAA, 0xBB].as_ref(), 64, 0xFF),
    )]
    fn test_bit_read(input: &[u8], read_bits: usize, expected: u32) {
        let bit_slice = input.bits::<Msb0>();
        let mut bit_index = 0usize;

        let res_read = u32::read(bit_slice, &mut bit_index, read_bits).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(bit_index, read_bits);
    }

    #[rstest(input,expected,
        case::normal(0xAABBCCDD, vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_bit_write(input: u32, expected: Vec<u8>) {
        let res_write = input.write();
        assert_eq!(expected, res_write);
    }

    #[rstest(input,read_bits,expected,expected_write,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 32, 0xAABBCCDD, vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_bit_read_write(input: &[u8], read_bits: usize, expected: u32, expected_write: Vec<u8>) {
        let bit_slice = input.bits::<Msb0>();
        let mut bit_index = 0usize;

        let res_read = u32::read(bit_slice, &mut bit_index, read_bits).unwrap();
        assert_eq!(expected, res_read);

        let res_write = res_read.write();
        assert_eq!(expected_write, res_write);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }

    #[test]
    fn test_swap_endian() {
        let input = 0xAABBCCDDu32;
        assert_eq!(0xDDCCBBAA, input.swap_endian());
    }
}
