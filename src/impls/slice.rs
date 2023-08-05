//! Implementations of DekuRead and DekuWrite for [T; N] where 0 < N <= 32

use bitvec::prelude::*;
pub use deku_derive::*;

use crate::ctx::Limit;
use crate::{DekuError, DekuRead, DekuWrite};

/// Read `u8`s and returns a byte slice up until a given predicate returns true
/// * `ctx` - The context required by `u8`. It will be passed to every `u8` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `u8`s
/// The predicate takes two parameters: the number of bits that have been read so far,
/// and a borrow of the latest value to have been read. It should return `true` if reading
/// should now stop, and `false` otherwise
fn read_slice_with_predicate<'a, Ctx, Predicate>(
    input: &'a BitSlice<u8, Msb0>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<(usize, &[u8]), DekuError>
where
    u8: DekuRead<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(usize, &u8) -> bool,
{
    let mut rest = input;
    let mut value;
    let mut total_read = 0;

    loop {
        let (amt_read, val) = u8::read(rest, ctx)?;
        rest = &rest[amt_read..];
        total_read += amt_read;

        let read_idx = unsafe { rest.as_bitptr().offset_from(input.as_bitptr()) } as usize;
        value = input[..read_idx].domain().region().unwrap().1;

        if predicate(read_idx, &val) {
            break;
        }
    }

    Ok((total_read, value))
}

impl<'a, Ctx, Predicate> DekuRead<'a, (Limit<u8, Predicate>, Ctx)> for &'a [u8]
where
    u8: DekuRead<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(&u8) -> bool,
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
    /// let (amt_read, v) = <&[u8]>::read(input.view_bits(), (4.into(), Endian::Little)).unwrap();
    /// assert_eq!(amt_read, 32);
    /// assert_eq!(&[1u8, 2, 3, 4], v)
    /// ```
    fn read(
        input: &'a BitSlice<u8, Msb0>,
        (limit, inner_ctx): (Limit<u8, Predicate>, Ctx),
    ) -> Result<(usize, Self), DekuError> {
        match limit {
            // Read a given count of elements
            Limit::Count(mut count) => {
                // Handle the trivial case of reading an empty slice
                if count == 0 {
                    return Ok((0, &input.domain().region().unwrap().1[..0]));
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

#[cfg(feature = "const_generics")]
mod const_generics_impl {
    use core::mem::MaybeUninit;
    use std::io::Read;

    use super::*;

    impl<'a, Ctx: Copy, T, const N: usize> DekuRead<'a, Ctx> for [T; N]
    where
        T: DekuRead<'a, Ctx>,
    {
        fn read(input: &'a BitSlice<u8, Msb0>, ctx: Ctx) -> Result<(usize, Self), DekuError>
        where
            Self: Sized,
        {
            #[allow(clippy::uninit_assumed_init)]
            // This is safe because we initialize the array immediately after,
            // and never return it in case of error
            let mut slice: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
            let mut rest = input;
            let mut total_read = 0;
            for (n, item) in slice.iter_mut().enumerate() {
                let (amt_read, value) = match T::read(rest, ctx) {
                    Ok(it) => it,
                    Err(err) => {
                        // For each item in the array, drop if we allocated it.
                        for item in &mut slice[0..n] {
                            unsafe {
                                item.assume_init_drop();
                            }
                        }
                        return Err(err);
                    }
                };
                item.write(value);
                rest = &rest[amt_read..];
                total_read += amt_read;
            }

            let val = unsafe {
                // TODO: array_assume_init: https://github.com/rust-lang/rust/issues/80908
                (&slice as *const _ as *const [T; N]).read()
            };
            Ok((total_read, val))
        }

        fn from_reader<R: Read>(
            container: &mut crate::container::Container<R>,
            ctx: Ctx,
        ) -> Result<Self, DekuError>
        where
            Self: Sized,
        {
            #[allow(clippy::uninit_assumed_init)]
            // This is safe because we initialize the array immediately after,
            // and never return it in case of error
            let mut slice: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
            for (n, item) in slice.iter_mut().enumerate() {
                let value = match T::from_reader(container, ctx) {
                    Ok(it) => it,
                    Err(err) => {
                        // For each item in the array, drop if we allocated it.
                        for item in &mut slice[0..n] {
                            unsafe {
                                item.assume_init_drop();
                            }
                        }
                        return Err(err);
                    }
                };
                item.write(value);
            }

            let val = unsafe {
                // TODO: array_assume_init: https://github.com/rust-lang/rust/issues/80908
                (&slice as *const _ as *const [T; N]).read()
            };
            Ok(val)
        }
    }

    impl<Ctx: Copy, T, const N: usize> DekuWrite<Ctx> for [T; N]
    where
        T: DekuWrite<Ctx>,
    {
        fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
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
        fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
            for v in *self {
                v.write(output, ctx)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::ctx::Endian;

    #[rstest(input,endian,expected,expected_rest,
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, [0xCCDD, 0xAABB], bits![u8, Msb0;]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, [0xDDCC, 0xBBAA], bits![u8, Msb0;]),
    )]
    fn test_bit_read(
        input: &[u8],
        endian: Endian,
        expected: [u16; 2],
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (amt_read, res_read) = <[u16; 2]>::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);
    }

    #[rstest(input,endian,expected,
        case::normal_le([0xDDCC, 0xBBAA], Endian::Little, vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xDDCC, 0xBBAA], Endian::Big, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_write(input: [u16; 2], endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());

        // test &slice
        let input = input.as_ref();
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }

    #[cfg(feature = "const_generics")]
    #[rstest(input,endian,expected,expected_rest,
        case::normal_le(
            [0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66].as_ref(),
            Endian::Little,
            [[0xCCDD, 0xAABB], [0x8899, 0x6677]],
            bits![u8, Msb0;],
        ),
        case::normal_le(
            [0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66].as_ref(),
            Endian::Big,
            [[0xDDCC, 0xBBAA], [0x9988, 0x7766]],
            bits![u8, Msb0;],
        ),
    )]
    fn test_nested_array_bit_read(
        input: &[u8],
        endian: Endian,
        expected: [[u16; 2]; 2],
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (amt_read, res_read) = <[[u16; 2]; 2]>::read(bit_slice, endian).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);
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
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());

        // test &slice
        let input = input.as_ref();
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }
}
