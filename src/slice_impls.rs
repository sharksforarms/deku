//! Implementations of DekuRead and DekuWrite for [T; N] where 0 < N <= 32

// Forked out DekuRead and DekuWrite impls for [T; N] into a separate file
// since the list of impls is long

use super::{DekuRead, DekuWrite};
use crate::error::DekuError;
use bitvec::prelude::*;
pub use deku_derive::*;

macro_rules! ImplDekuSliceTraits {
    ($typ:ty; $($count:expr),+ $(,)?) => { $(
        impl<Ctx: Copy> DekuRead<Ctx> for [$typ; $count]
        where
            $typ: DekuRead<Ctx>,
        {
            fn read(
                input: &BitSlice<Msb0, u8>,
                ctx: Ctx,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
            where
                Self: Sized,
            {
                let mut slice: [$typ; $count] = Default::default();
                let mut rest = input;
                for i in 0..$count {
                    let (new_rest, value) = <$typ>::read(rest, ctx)?;
                    slice[i] = value;
                    rest = new_rest;
                }

                Ok((rest, slice))
            }
        }

        impl<Ctx: Copy> DekuWrite<Ctx> for [$typ; $count]
        where
            $typ: DekuWrite<Ctx>,
        {
            fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
                for v in self {
                    v.write(output, ctx)?;
                }
                Ok(())
            }
        }
    )+ };
}

/*
Generate the list with:

```python
TYPES = [
    'u8', 'u16', 'u32', 'u64', 'u128', 'usize',
    'i8', 'i16', 'i32', 'i64', 'i128', 'isize',
    'f32', 'f64',
]
MAX_SIZE = 32

for typ in TYPES:
    for size in range(1, MAX_SIZE + 1):
        print(f"ImplDekuSliceTraits!({typ}, {size});")
```
*/

ImplDekuSliceTraits!(i8; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(i16; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(i32; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(i64; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(i128; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(isize; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(u8; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(u16; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(u32; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(u64; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(u128; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(usize; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(f32; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
ImplDekuSliceTraits!(f64; 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ctx::Endian;
    use rstest::rstest;

    #[rstest(input,endian,expected,expected_rest,
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, [0xCCDD, 0xAABB], bits![Msb0, u8;]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, [0xDDCC, 0xBBAA], bits![Msb0, u8;]),
    )]
    fn test_bit_read(
        input: &[u8],
        endian: Endian,
        expected: [u16; 2],
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = <[u16; 2]>::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input,endian,expected,
        case::normal_le([0xDDCC, 0xBBAA], Endian::Little, vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xDDCC, 0xBBAA], Endian::Big, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_write(input: [u16; 2], endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![Msb0, u8;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }
}
