//! Implementations of DekuRead and DekuWrite for [T; N] where 0 < N <= 32

use crate::{ctx::Limit, DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;
pub use deku_derive::*;

/// Read `u8`s and returns a byte slice up until a given predicate returns true
/// * `ctx` - The context required by `u8`. It will be passed to every `u8` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `u8`s
/// The predicate takes two parameters: the number of bits that have been read so far,
/// and a borrow of the latest value to have been read. It should return `true` if reading
/// should now stop, and `false` otherwise
fn read_slice_with_predicate<'a, Ctx: Copy, Predicate: FnMut(usize, &u8) -> bool>(
    input: &'a BitSlice<Msb0, u8>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<(&'a BitSlice<Msb0, u8>, &[u8]), DekuError>
where
    u8: DekuRead<'a, Ctx>,
{
    let mut rest = input;
    let mut value;

    loop {
        let (new_rest, val) = u8::read(rest, ctx)?;
        rest = new_rest;

        let read_idx = input.offset_from(rest) as usize;
        value = input[..read_idx].as_raw_slice();

        if predicate(read_idx, &val) {
            break;
        }
    }

    Ok((rest, value))
}

impl<'a, Ctx: Copy, Predicate: FnMut(&u8) -> bool> DekuRead<'a, (Limit<u8, Predicate>, Ctx)>
    for &'a [u8]
where
    u8: DekuRead<'a, Ctx>,
{
    /// Read `u8`s until the given limit
    /// * `limit` - the limiting factor on the amount of `u8`s to read
    /// * `inner_ctx` - The context required by `u8`. It will be passed to every `u8`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use bitvec::view::BitView;
    /// let input = vec![1u8, 2, 3, 4];
    /// let (rest, v) = <&[u8]>::read(input.view_bits(), (4.into(), Endian::Little)).unwrap();
    /// assert!(rest.is_empty());
    /// assert_eq!(&[1u8, 2, 3, 4], v)
    /// ```
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        (limit, inner_ctx): (Limit<u8, Predicate>, Ctx),
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError> {
        match limit {
            // Read a given count of elements
            Limit::Count(mut count) => {
                // Handle the trivial case of reading an empty slice
                if count == 0 {
                    return Ok((input, &input.as_raw_slice()[..0]));
                }

                // Otherwise, read until we have read `count` elements
                read_slice_with_predicate(input, inner_ctx, move |_, _| {
                    count -= 1;
                    count == 0
                })
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => {
                read_slice_with_predicate(input, inner_ctx, move |_, value| predicate(value))
            }

            // Read until a given quantity of bits have been read
            Limit::BitSize(size) => {
                let bit_size = size.0;
                read_slice_with_predicate(input, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }

            // Read until a given quantity of bytes have been read
            Limit::ByteSize(size) => {
                let bit_size = size.0 * 8;
                read_slice_with_predicate(input, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }
        }
    }
}

#[cfg(not(feature = "const_generics"))]
mod pre_const_generics_impl {
    use super::*;

    macro_rules! ImplDekuSliceTraits {
        ($typ:ty; $($count:expr),+ $(,)?) => {

            impl<Ctx: Copy> DekuWrite<Ctx> for &[$typ]
            where
                $typ: DekuWrite<Ctx>,
            {
                fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
                    for v in *self {
                        v.write(output, ctx)?;
                    }
                    Ok(())
                }
            }

            $(
                impl<'a, Ctx: Copy> DekuRead<'a, Ctx> for [$typ; $count]
                where
                    $typ: DekuRead<'a, Ctx>,
                {
                    fn read(
                        input: &'a BitSlice<Msb0, u8>,
                        ctx: Ctx,
                    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
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
            )+
        };
    }

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
}

#[cfg(feature = "const_generics")]
mod const_generics_impl {
    use super::*;

    use core::mem::MaybeUninit;

    impl<'a, Ctx: Copy, T, const N: usize> DekuRead<'a, Ctx> for [T; N]
    where
        T: DekuRead<'a, Ctx>,
    {
        fn read(
            input: &'a BitSlice<Msb0, u8>,
            ctx: Ctx,
        ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
        where
            Self: Sized,
        {
            #[allow(clippy::uninit_assumed_init)]
            // This is safe because we initialize the array immediately after,
            // and never return it in case of error
            let mut slice: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
            let mut rest = input;
            for item in slice.iter_mut() {
                let (new_rest, value) = T::read(rest, ctx)?;
                item.write(value);
                rest = new_rest;
            }

            Ok((rest, unsafe {
                // TODO: array_assume_init: https://github.com/rust-lang/rust/issues/80908
                (&slice as *const _ as *const [T; N]).read()
            }))
        }
    }

    impl<Ctx: Copy, T, const N: usize> DekuWrite<Ctx> for [T; N]
    where
        T: DekuWrite<Ctx>,
    {
        fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
            for v in self {
                v.write(output, ctx)?;
            }
            Ok(())
        }
    }

    impl<Ctx: Copy, T> DekuWrite<Ctx> for &[T]
    where
        T: DekuWrite<Ctx>,
    {
        fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
            for v in *self {
                v.write(output, ctx)?;
            }
            Ok(())
        }
    }
}

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

        // test &slice
        let input = input.as_ref();
        let mut res_write = bitvec![Msb0, u8;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }

    #[cfg(feature = "const_generics")]
    #[rstest(input,endian,expected,expected_rest,
        case::normal_le(
            [0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66].as_ref(),
            Endian::Little,
            [[0xCCDD, 0xAABB], [0x8899, 0x6677]],
            bits![Msb0, u8;],
        ),
        case::normal_le(
            [0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66].as_ref(),
            Endian::Big,
            [[0xDDCC, 0xBBAA], [0x9988, 0x7766]],
            bits![Msb0, u8;],
        ),
    )]
    fn test_nested_array_bit_read(
        input: &[u8],
        endian: Endian,
        expected: [[u16; 2]; 2],
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = <[[u16; 2]; 2]>::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[cfg(feature = "const_generics")]
    #[rstest(input,endian,expected,
        case::normal_le(
            [[0xCCDD, 0xAABB], [0x8899, 0x6677]],
            Endian::Little,
            vec![0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66],
        ),
        case::normal_be(
            [[0xDDCC, 0xBBAA], [0x9988, 0x7766]],
            Endian::Big,
            vec![0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66],
        ),
    )]
    fn test_nested_array_bit_write(input: [[u16; 2]; 2], endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![Msb0, u8;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());

        // test &slice
        let input = input.as_ref();
        let mut res_write = bitvec![Msb0, u8;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }
}
