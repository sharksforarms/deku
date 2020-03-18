pub use deku_derive::*;
use nom::{bits, IResult};

pub trait BitsSize {
    fn bit_size() -> usize;
}

pub trait BitsReader: BitsSize {
    fn read(input: (&[u8], usize), bits: usize) -> ((&[u8], usize), Self);
}

pub trait BitsWriter: BitsSize {
    fn write(self) -> Vec<u8>;
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
                let res = self.to_le_bytes();

                #[cfg(target_endian = "big")]
                let res = self.to_be_bytes();

                res.to_vec()
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
