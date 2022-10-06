use crate::{ctx::*, DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Read `T`s into a vec until a given predicate returns true
/// * `capacity` - an optional capacity to pre-allocate the vector with
/// * `ctx` - The context required by `T`. It will be passed to every `T` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `T`s
/// The predicate takes two parameters: the number of bits that have been read so far,
/// and a borrow of the latest value to have been read. It should return `true` if reading
/// should now stop, and `false` otherwise
fn read_vec_with_predicate<
    'a,
    T: DekuRead<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(usize, &T) -> bool,
>(
    input: &'a BitSlice<Msb0, u8>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<(&'a BitSlice<Msb0, u8>, Vec<T>), DekuError> {
    let mut res = capacity.map_or_else(Vec::new, Vec::with_capacity);

    let mut rest = input;

    loop {
        let (new_rest, val) = <T>::read(rest, ctx)?;
        res.push(val);
        rest = new_rest;

        // This unwrap is safe as we are pushing to the vec immediately before it,
        // so there will always be a last element
        if predicate(input.offset_from(rest) as usize, res.last().unwrap()) {
            break;
        }
    }

    Ok((rest, res))
}

impl<'a, T: DekuRead<'a, Ctx>, Ctx: Copy, Predicate: FnMut(&T) -> bool>
    DekuRead<'a, (Limit<T, Predicate>, Ctx)> for Vec<T>
{
    /// Read `T`s until the given limit
    /// * `limit` - the limiting factor on the amount of `T`s to read
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use deku::bitvec::BitView;
    /// let input = vec![1u8, 2, 3, 4];
    /// let (rest, v) = Vec::<u32>::read(input.view_bits(), (1.into(), Endian::Little)).unwrap();
    /// assert!(rest.is_empty());
    /// assert_eq!(vec![0x04030201], v)
    /// ```
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        (limit, inner_ctx): (Limit<T, Predicate>, Ctx),
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        match limit {
            // Read a given count of elements
            Limit::Count(mut count) => {
                // Handle the trivial case of reading an empty vector
                if count == 0 {
                    return Ok((input, Vec::new()));
                }

                // Otherwise, read until we have read `count` elements
                read_vec_with_predicate(input, Some(count), inner_ctx, move |_, _| {
                    count -= 1;
                    count == 0
                })
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => {
                read_vec_with_predicate(input, None, inner_ctx, move |_, value| predicate(value))
            }

            // Read until a given quantity of bits have been read
            Limit::BitSize(size) => {
                let bit_size = size.0;
                read_vec_with_predicate(input, None, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }

            // Read until a given quantity of bits have been read
            Limit::ByteSize(size) => {
                let bit_size = size.0 * 8;
                read_vec_with_predicate(input, None, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }
        }
    }
}

impl<'a, T: DekuRead<'a>, Predicate: FnMut(&T) -> bool> DekuRead<'a, Limit<T, Predicate>>
    for Vec<T>
{
    /// Read `T`s until the given limit from input for types which don't require context.
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        limit: Limit<T, Predicate>,
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        Vec::read(input, (limit, ()))
    }
}

impl<T: DekuWrite<Ctx>, Ctx: Copy> DekuWrite<Ctx> for Vec<T> {
    /// Write all `T`s in a `Vec` to bits.
    /// * **inner_ctx** - The context required by `T`.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite};
    /// # use deku::bitvec::{Msb0, bitvec};
    /// let data = vec![1u8];
    /// let mut output = bitvec![Msb0, u8;];
    /// data.write(&mut output, Endian::Big).unwrap();
    /// assert_eq!(output, bitvec![Msb0, u8; 0, 0, 0, 0, 0, 0, 0, 1])
    /// ```
    fn write(&self, output: &mut BitVec<Msb0, u8>, inner_ctx: Ctx) -> Result<(), DekuError> {
        for v in self {
            v.write(output, inner_ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest(input,endian,bit_size,limit,expected,expected_rest,
        case::count_0([0xAA].as_ref(), Endian::Little, Some(8), 0.into(), vec![], bits![Msb0, u8; 1, 0, 1, 0, 1, 0, 1, 0]),
        case::count_1([0xAA, 0xBB].as_ref(), Endian::Little, Some(8), 1.into(), vec![0xAA], bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), Endian::Little, Some(8), 2.into(), vec![0xAA, 0xBB], bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0]),
        case::until_null([0xAA, 0, 0xBB].as_ref(), Endian::Little, None, (|v: &u8| *v == 0u8).into(), vec![0xAA, 0], bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::until_bits([0xAA, 0xBB].as_ref(), Endian::Little, None, BitSize(8).into(), vec![0xAA], bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::bits_6([0b0110_1001, 0b1110_1001].as_ref(), Endian::Little, Some(6), 2.into(), vec![0b00_011010, 0b00_011110], bits![Msb0, u8; 1, 0, 0, 1]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(9), 1.into(), vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(9), 1.into(), vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(8), 2.into(), vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_until([0xAA].as_ref(), Endian::Little, Some(8), (|_: &u8| false).into(), vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_bits([0xAA].as_ref(), Endian::Little, Some(8), (BitSize(16)).into(), vec![], bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(9), 1.into(), vec![], bits![Msb0, u8;]),
    )]
    fn test_vec_read<Predicate: FnMut(&u8) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u8, Predicate>,
        expected: Vec<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => {
                Vec::<u8>::read(bit_slice, (limit, (endian, BitSize(bit_size)))).unwrap()
            }
            None => Vec::<u8>::read(bit_slice, (limit, (endian))).unwrap(),
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

    // Note: These tests also exist in boxed.rs
    #[rstest(input, endian, bit_size, limit, expected, expected_rest, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC], bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD], bits![Msb0, u8;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA], bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB], bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), vec![0xBBAA], bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), vec![0xAABB], bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
    )]
    fn test_vec_read_write<Predicate: FnMut(&u16) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: Vec<u16>,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let (rest, res_read) =
            Vec::<u16>::read(bit_slice, (limit, (endian, BitSize(bit_size)))).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read
            .write(&mut res_write, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, res_write.into_vec());

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }
}
