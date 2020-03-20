pub use deku_derive::*;
use nom::{bits, IResult};

pub mod prelude;

pub trait BitsSize {
    fn bit_size() -> usize;
}

pub trait BitsReader: BitsSize {
    fn read(input: (&[u8], usize), bits: usize) -> ((&[u8], usize), Self);
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
            fn read(input: (&[u8], usize), bits: usize) -> ((&[u8], usize), Self) {
                fn parser(input: (&[u8], usize), bits: usize) -> IResult<(&[u8], usize), $typ> {
                    bits::complete::take(bits)(input)
                }

                let res = parser(input, bits).unwrap();
                res
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
ImplDekuTraits!(u128);
ImplDekuTraits!(usize);

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[test]
    fn test_bit_size() {
        assert_eq!(8, u8::bit_size());
        assert_eq!(128, u128::bit_size());
    }

    #[rstest(input,read_bits,expected,expected_rest,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 32, 0xAABBCCDD, ([].as_ref(), 0)),
        case::normal_offset([0b1001_0110, 0b1110_0000, 0xCC, 0xDD].as_ref(), 12, 0b1001_0110_1110, ([0b1110_0000, 0xCC, 0xDD].as_ref(), 4)),
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF].as_ref(), 32, 0xAABBCCDD, ([0xEE, 0xFF].as_ref(), 0)),

        // TODO: Better error message for these
        #[should_panic(expected="Error((([170, 187], 0), Eof))")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), 32, 0xFF, ([].as_ref(), 0)),
    )]
    fn test_bit_read(input: &[u8], read_bits: usize, expected: u32, expected_rest: (&[u8], usize)) {
        let res_read = u32::read((input, 0usize), read_bits);
        assert_eq!(expected, res_read.1);
        assert_eq!(expected_rest, res_read.0);
    }

    #[rstest(input,expected,
        case::normal(0xAABBCCDD, vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_bit_write(input: u32, expected: Vec<u8>) {
        let res_write = input.write();
        assert_eq!(expected, res_write);
    }

    #[rstest(input,read_bits,expected,expected_rest,expected_write,
        case::normal([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), 32, 0xAABBCCDD, ([].as_ref(), 0), vec![0xAA, 0xBB, 0xCC, 0xDD]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        read_bits: usize,
        expected: u32,
        expected_rest: (&[u8], usize),
        expected_write: Vec<u8>,
    ) {
        let res_read = u32::read((input, 0usize), read_bits);
        assert_eq!(expected, res_read.1);
        assert_eq!(expected_rest, res_read.0);

        let res_write = res_read.1.write();
        assert_eq!(expected_write, res_write);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }

    #[test]
    fn test_swap_endian() {
        let input = 0xAABBCCDDu32;
        assert_eq!(0xDDCCBBAA, input.swap_endian());
    }
}
