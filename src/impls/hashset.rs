use crate::{ctx::*, DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;
use std::collections::HashSet;
use std::hash::{BuildHasher, Hash};

/// Read `T`s into a hashset until a given predicate returns true
/// * `capacity` - an optional capacity to pre-allocate the hashset with
/// * `ctx` - The context required by `T`. It will be passed to every `T` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `T`s
/// The predicate takes two parameters: the number of bits that have been read so far,
/// and a borrow of the latest value to have been read. It should return `true` if reading
/// should now stop, and `false` otherwise
#[allow(clippy::type_complexity)]
fn read_hashset_with_predicate<
    'a,
    T: DekuRead<'a, Ctx> + Eq + Hash,
    S: BuildHasher + Default,
    Ctx: Copy,
    Predicate: FnMut(usize, &T) -> bool,
>(
    input: &'a BitSlice<Msb0, u8>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<(&'a BitSlice<Msb0, u8>, HashSet<T, S>), DekuError> {
    let mut res = HashSet::with_capacity_and_hasher(capacity.unwrap_or(0), S::default());

    let mut rest = input;
    let mut found_predicate = false;

    while !found_predicate {
        let (new_rest, val) = <T>::read(rest, ctx)?;
        found_predicate = predicate(input.offset_from(new_rest) as usize, &val);
        res.insert(val);
        rest = new_rest;
    }

    Ok((rest, res))
}

impl<
        'a,
        T: DekuRead<'a, Ctx> + Eq + Hash,
        S: BuildHasher + Default,
        Ctx: Copy,
        Predicate: FnMut(&T) -> bool,
    > DekuRead<'a, (Limit<T, Predicate>, Ctx)> for HashSet<T, S>
{
    /// Read `T`s until the given limit
    /// * `limit` - the limiting factor on the amount of `T`s to read
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use deku::bitvec::BitView;
    /// # use std::collections::HashSet;
    /// let input = vec![1u8, 2, 3, 4];
    /// let expected: HashSet<u32> = vec![0x04030201].into_iter().collect();
    /// let (rest, set) = HashSet::<u32>::read(input.view_bits(), (1.into(), Endian::Little)).unwrap();
    /// assert!(rest.is_empty());
    /// assert_eq!(expected, set)
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
                // Handle the trivial case of reading an empty hashset
                if count == 0 {
                    return Ok((input, HashSet::<T, S>::default()));
                }

                // Otherwise, read until we have read `count` elements
                read_hashset_with_predicate(input, Some(count), inner_ctx, move |_, _| {
                    count -= 1;
                    count == 0
                })
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => {
                read_hashset_with_predicate(input, None, inner_ctx, move |_, value| {
                    predicate(value)
                })
            }

            // Read until a given quantity of bits have been read
            Limit::Size(size) => {
                let bit_size = size.bit_size();
                read_hashset_with_predicate(input, None, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }
        }
    }
}

impl<'a, T: DekuRead<'a> + Eq + Hash, S: BuildHasher + Default, Predicate: FnMut(&T) -> bool>
    DekuRead<'a, Limit<T, Predicate>> for HashSet<T, S>
{
    /// Read `T`s until the given limit from input for types which don't require context.
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        limit: Limit<T, Predicate>,
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        Self::read(input, (limit, ()))
    }
}

impl<T: DekuWrite<Ctx>, S, Ctx: Copy> DekuWrite<Ctx> for HashSet<T, S> {
    /// Write all `T`s in a `HashSet` to bits.
    /// * **inner_ctx** - The context required by `T`.
    /// Note: depending on the Hasher `S`, the order in which the `T`'s are
    /// written may change between executions. Use a deterministic Hasher for your HashSet
    /// instead of the default RandomState hasher if you don't want the order written to change.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite};
    /// # use deku::bitvec::{Msb0, bitvec};
    /// # use std::collections::HashSet;
    /// let set: HashSet<u8> = vec![1].into_iter().collect();
    /// let mut output = bitvec![Msb0, u8;];
    /// set.write(&mut output, Endian::Big).unwrap();
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
    use rustc_hash::FxHashSet;

    #[rstest(input, endian, bit_size, limit, expected, expected_rest,
        case::count_0([0xAA].as_ref(), Endian::Little, Some(8), 0.into(), FxHashSet::default(), bits![Msb0, u8; 1, 0, 1, 0, 1, 0, 1, 0]),
        case::count_1([0xAA, 0xBB].as_ref(), Endian::Little, Some(8), 1.into(), vec![0xAA].into_iter().collect(), bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), Endian::Little, Some(8), 2.into(), vec![0xAA, 0xBB].into_iter().collect(), bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0]),
        case::until_null([0xAA, 0, 0xBB].as_ref(), Endian::Little, None, (|v: &u8| *v == 0u8).into(), vec![0xAA, 0].into_iter().collect(), bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::until_bits([0xAA, 0xBB].as_ref(), Endian::Little, None, Size::Bits(8).into(), vec![0xAA].into_iter().collect(), bits![Msb0, u8; 1, 0, 1, 1, 1, 0, 1, 1]),
        case::bits_6([0b0110_1001, 0b1110_1001].as_ref(), Endian::Little, Some(6), 2.into(), vec![0b00_011010, 0b00_011110].into_iter().collect(), bits![Msb0, u8; 1, 0, 0, 1]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(9), 1.into(), FxHashSet::default(), bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(9), 1.into(), FxHashSet::default(), bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(8), 2.into(), FxHashSet::default(), bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_until([0xAA].as_ref(), Endian::Little, Some(8), (|_: &u8| false).into(), FxHashSet::default(), bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_bits([0xAA].as_ref(), Endian::Little, Some(8), (Size::Bits(16)).into(), FxHashSet::default(), bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(9), 1.into(), FxHashSet::default(), bits![Msb0, u8;]),
    )]
    fn test_hashset_read<Predicate: FnMut(&u8) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u8, Predicate>,
        expected: FxHashSet<u8>,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => {
                FxHashSet::<u8>::read(bit_slice, (limit, (endian, Size::Bits(bit_size)))).unwrap()
            }
            None => FxHashSet::<u8>::read(bit_slice, (limit, (endian))).unwrap(),
        };

        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
    }

    #[rstest(input, endian, expected,
        case::normal(vec![0xAABB, 0xCCDD].into_iter().collect(), Endian::Little, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_hashset_write(input: FxHashSet<u16>, endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![Msb0, u8;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }

    // Note: These tests also exist in boxed.rs
    #[rstest(input, endian, bit_size, limit, expected, expected_rest, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC].into_iter().collect(), bits![Msb0, u8;], vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD].into_iter().collect(), bits![Msb0, u8;], vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA].into_iter().collect(), bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB].into_iter().collect(), bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), Size::Bits(16).into(), vec![0xBBAA].into_iter().collect(), bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), Size::Bits(16).into(), vec![0xAABB].into_iter().collect(), bits![Msb0, u8; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
    )]
    fn test_hashset_read_write<Predicate: FnMut(&u16) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: FxHashSet<u16>,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let (rest, res_read) =
            FxHashSet::<u16>::read(bit_slice, (limit, (endian, Size::Bits(bit_size)))).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read
            .write(&mut res_write, (endian, Size::Bits(bit_size)))
            .unwrap();
        assert_eq!(expected_write, res_write.into_vec());
    }
}
