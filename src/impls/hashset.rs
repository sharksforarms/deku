use core::hash::{BuildHasher, Hash};
use std::collections::HashSet;

use crate::writer::Writer;
use no_std_io::io::{Read, Seek, Write};

use crate::ctx::*;
use crate::{DekuError, DekuReader, DekuWriter};

/// Read `T`s into a hashset until a given predicate returns true
/// * `capacity` - an optional capacity to pre-allocate the hashset with
/// * `ctx` - The context required by `T`. It will be passed to every `T` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `T`s
///   The predicate takes two parameters: the number of bits that have been read so far,
///   and a borrow of the latest value to have been read. It should return `true` if reading
///   should now stop, and `false` otherwise
#[allow(clippy::type_complexity)]
fn from_reader_with_ctx_hashset_with_predicate<'a, T, S, Ctx, Predicate, R: Read + Seek>(
    reader: &mut crate::reader::Reader<R>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<HashSet<T, S>, DekuError>
where
    T: DekuReader<'a, Ctx> + Eq + Hash,
    S: BuildHasher + Default,
    Ctx: Clone,
    Predicate: FnMut(usize, &T) -> bool,
{
    let mut res = HashSet::with_capacity_and_hasher(capacity.unwrap_or(0), S::default());

    let mut found_predicate = false;
    let orig_bits_read = reader.bits_read;

    while !found_predicate {
        let val = <T>::from_reader_with_ctx(reader, ctx.clone())?;
        found_predicate = predicate(reader.bits_read - orig_bits_read, &val);
        res.insert(val);
    }

    Ok(res)
}

fn from_reader_with_ctx_hashset_to_end<'a, T, S, Ctx, R: Read + Seek>(
    reader: &mut crate::reader::Reader<R>,
    capacity: Option<usize>,
    ctx: Ctx,
) -> Result<HashSet<T, S>, DekuError>
where
    T: DekuReader<'a, Ctx> + Eq + Hash,
    S: BuildHasher + Default,
    Ctx: Clone,
{
    let mut res = HashSet::with_capacity_and_hasher(capacity.unwrap_or(0), S::default());

    loop {
        if reader.end() {
            break;
        }
        let val = <T>::from_reader_with_ctx(reader, ctx.clone())?;
        res.insert(val);
    }

    Ok(res)
}

impl<'a, T, S, Ctx, Predicate, PredicateWithCtx>
    DekuReader<'a, (Limit<T, Predicate, Ctx, PredicateWithCtx>, Ctx)> for HashSet<T, S>
where
    T: DekuReader<'a, Ctx> + Eq + Hash,
    S: BuildHasher + Default,
    Ctx: Clone,
    Predicate: FnMut(&T) -> bool,
    PredicateWithCtx: FnMut(&T, Ctx) -> bool,
{
    /// Read `T`s until the given limit
    /// * `limit` - the limiting factor on the amount of `T`s to read
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuReader;
    /// # use std::collections::HashSet;
    /// # use std::io::Cursor;
    /// let mut input = Cursor::new(vec![1u8, 2, 3, 4]);
    /// let expected: HashSet<u32> = vec![0x04030201].into_iter().collect();
    /// let mut reader = deku::reader::Reader::new(&mut input);
    /// let set = HashSet::<u32>::from_reader_with_ctx(&mut reader, (1.into(), Endian::Little)).unwrap();
    /// assert_eq!(expected, set)
    /// ```
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut crate::reader::Reader<R>,
        (limit, inner_ctx): (Limit<T, Predicate, Ctx, PredicateWithCtx>, Ctx),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        match limit {
            // Read a given count of elements
            Limit::Count(mut count) => {
                // Handle the trivial case of reading an empty hashset
                if count == 0 {
                    return Ok(HashSet::<T, S>::default());
                }

                // Otherwise, read until we have read `count` elements
                from_reader_with_ctx_hashset_with_predicate(
                    reader,
                    Some(count),
                    inner_ctx.clone(),
                    move |_, _| {
                        count -= 1;
                        count == 0
                    },
                )
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => from_reader_with_ctx_hashset_with_predicate(
                reader,
                None,
                inner_ctx.clone(),
                move |_, value| predicate(value),
            ),

            // Read until a given predicate returns true
            Limit::UntilWithCtx(mut predicate, _, _) => {
                from_reader_with_ctx_hashset_with_predicate(
                    reader,
                    None,
                    inner_ctx.clone(),
                    move |_, value| predicate(value, inner_ctx.clone()),
                )
            }

            // Read until a given quantity of bits have been read
            Limit::BitSize(size) => {
                let bit_size = size.0;

                // Handle the trivial case of reading an empty hashset
                if bit_size == 0 {
                    return Ok(HashSet::<T, S>::default());
                }

                from_reader_with_ctx_hashset_with_predicate(
                    reader,
                    None,
                    inner_ctx.clone(),
                    move |read_bits, _| read_bits == bit_size,
                )
            }

            // Read until a given quantity of bytes have been read
            Limit::ByteSize(size) => {
                let bit_size = size.0 * 8;

                // Handle the trivial case of reading an empty hashset
                if bit_size == 0 {
                    return Ok(HashSet::<T, S>::default());
                }

                from_reader_with_ctx_hashset_with_predicate(
                    reader,
                    None,
                    inner_ctx.clone(),
                    move |read_bits, _| read_bits == bit_size,
                )
            }

            // Read until `reader.end()` is true
            Limit::End => from_reader_with_ctx_hashset_to_end(reader, None, inner_ctx.clone()),
        }
    }
}

impl<
        'a,
        T: DekuReader<'a> + Eq + Hash,
        S: BuildHasher + Default,
        Predicate: FnMut(&T) -> bool,
        PredicateWithCtx: FnMut(&T, ()) -> bool,
    > DekuReader<'a, Limit<T, Predicate, (), PredicateWithCtx>> for HashSet<T, S>
{
    /// Read `T`s until the given limit from input for types which don't require context.
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut crate::reader::Reader<R>,
        limit: Limit<T, Predicate, (), PredicateWithCtx>,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        Self::from_reader_with_ctx(reader, (limit, ()))
    }
}

impl<T: DekuWriter<Ctx>, S, Ctx: Clone> DekuWriter<Ctx> for HashSet<T, S> {
    /// Write all `T`s in a `HashSet` to bits.
    /// * **inner_ctx** - The context required by `T`.
    ///
    /// Note: depending on the Hasher `S`, the order in which the `T`'s are
    /// written may change between executions. Use a deterministic Hasher for your HashSet
    /// instead of the default RandomState hasher if you don't want the order written to change.
    ///
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWriter};
    /// # use deku::writer::Writer;
    /// # #[cfg(feature = "bits")]
    /// # use deku::bitvec::{Msb0, bitvec};
    /// # use std::collections::HashSet;
    /// # use std::io::Cursor;
    /// let mut out_buf = vec![];
    /// let mut cursor = Cursor::new(&mut out_buf);
    /// let mut writer = Writer::new(&mut cursor);
    /// let set: HashSet<u8> = vec![1].into_iter().collect();
    /// set.to_writer(&mut writer, Endian::Big).unwrap();
    /// assert_eq!(out_buf, vec![1]);
    /// ```
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        for v in self {
            v.to_writer(writer, inner_ctx.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    #[cfg(feature = "bits")]
    use crate::bitvec::{bits, BitSlice, Msb0};
    use no_std_io::io::Cursor;
    use rstest::rstest;
    use rustc_hash::FxHashSet;

    #[cfg(feature = "bits")]
    use crate::reader::Reader;

    use super::*;

    type MyLimit<Predicate> =
        Limit<u8, Predicate, (Endian, BitSize), fn(&u8, (Endian, BitSize)) -> bool>;

    #[cfg(all(feature = "bits", feature = "descriptive-errors"))]
    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes,
        case::count_0([0xAA].as_ref(), Endian::Little, Some(8), 0.into(), FxHashSet::default(), bits![u8, Msb0;], &[0xaa]),
        case::count_1([0xAA, 0xBB].as_ref(), Endian::Little, Some(8), 1.into(), vec![0xAA].into_iter().collect(), bits![u8, Msb0;], &[0xbb]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), Endian::Little, Some(8), 2.into(), vec![0xAA, 0xBB].into_iter().collect(), bits![u8, Msb0;], &[0xcc]),
        case::bits_6([0b0110_1001, 0b1110_1001].as_ref(), Endian::Little, Some(6), 2.into(), vec![0b00_011010, 0b00_011110].into_iter().collect(), bits![u8, Msb0; 1, 0, 0, 1], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(9), 1.into(), FxHashSet::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(9), 1.into(), FxHashSet::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(8), 2.into(), FxHashSet::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_until([0xAA].as_ref(), Endian::Little, Some(8), (|_: &u8| false).into(), FxHashSet::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_bits([0xAA].as_ref(), Endian::Little, Some(8), (BitSize(16)).into(), FxHashSet::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(9), 1.into(), FxHashSet::default(), bits![u8, Msb0;], &[]),
    )]
    fn test_hashset_read<Predicate: FnMut(&u8) -> bool + Copy>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: MyLimit<Predicate>,
        expected: FxHashSet<u8>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = FxHashSet::<u8>::from_reader_with_ctx(
            &mut reader,
            (limit, (endian, BitSize(bit_size.unwrap()))),
        )
        .unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    type MyLimit2<Predicate> = Limit<u8, Predicate, Endian, fn(&u8, Endian) -> bool>;

    #[cfg(all(feature = "bits", feature = "descriptive-errors"))]
    #[rstest(input, endian, limit, expected, expected_rest_bits, expected_rest_bytes,
        case::until_null([0xAA, 0, 0xBB].as_ref(), Endian::Little, (|v: &u8| *v == 0u8).into(), vec![0xAA, 0].into_iter().collect(), bits![u8, Msb0;], &[0xbb]),
        case::until_empty_bits([0xAA, 0xBB].as_ref(), Endian::Little, BitSize(0).into(), HashSet::default(), bits![u8, Msb0;], &[0xaa, 0xbb]),
        case::until_empty_bytes([0xAA, 0xBB].as_ref(), Endian::Little, ByteSize(0).into(), HashSet::default(), bits![u8, Msb0;], &[0xaa, 0xbb]),
        case::until_bits([0xAA, 0xBB].as_ref(), Endian::Little, BitSize(8).into(), vec![0xAA].into_iter().collect(), bits![u8, Msb0;], &[0xbb]),
        case::read_all([0xAA, 0xBB].as_ref(), Endian::Little, Limit::end(), vec![0xAA, 0xBB].into_iter().collect(), bits![u8, Msb0;], &[]),
        case::until_bytes([0xAA, 0xBB].as_ref(), Endian::Little, ByteSize(1).into(), vec![0xAA].into_iter().collect(), bits![u8, Msb0;], &[0xbb]),
        case::until_count([0xAA, 0xBB].as_ref(), Endian::Little, Limit::from(1), vec![0xAA].into_iter().collect(), bits![u8, Msb0;], &[0xbb]),
    )]
    fn test_hashset_read_no_bitsize<Predicate: FnMut(&u8) -> bool + Copy>(
        input: &[u8],
        endian: Endian,
        limit: MyLimit2<Predicate>,
        expected: FxHashSet<u8>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read =
            FxHashSet::<u8>::from_reader_with_ctx(&mut reader, (limit, (endian))).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[rstest(input, endian, expected,
        case::normal(vec![0xAABB, 0xCCDD].into_iter().collect(), Endian::Little, vec![0xBB, 0xAA, 0xDD, 0xCC]),
    )]
    fn test_hashset_write(input: FxHashSet<u16>, endian: Endian, expected: Vec<u8>) {
        let mut writer = Writer::new(Cursor::new(vec![]));
        input.to_writer(&mut writer, endian).unwrap();
        assert!(writer
            .inner
            .into_inner()
            .as_slice()
            .chunks(core::mem::size_of::<u16>())
            .all(|v| expected
                .as_slice()
                .chunks(core::mem::size_of::<u16>())
                .any(|u| v == u)));
    }

    // Limit<u16, fn(&u16), (Endian, BitSize), fn(&u16, (Endian, BitSize))>
    //::from(2)
    // Note: These tests also exist in boxed.rs
    #[cfg(feature = "bits")]
    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC].into_iter().collect(), bits![u8, Msb0;], &[], vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD].into_iter().collect(), bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA].into_iter().collect(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB].into_iter().collect(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), vec![0xBBAA].into_iter().collect(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), vec![0xAABB].into_iter().collect(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
    )]
    fn test_hashset_read_write<Predicate: FnMut(&u16) -> bool + Copy>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate, (Endian, BitSize), fn(&u16, (Endian, BitSize)) -> bool>,
        expected: FxHashSet<u16>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
        expected_write: Vec<u8>,
    ) {
        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = FxHashSet::<u16>::from_reader_with_ctx(
            &mut reader,
            (limit, (endian, BitSize(bit_size))),
        )
        .unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read
            .to_writer(&mut writer, (endian, BitSize(bit_size)))
            .unwrap();
        assert!(writer
            .inner
            .into_inner()
            .as_slice()
            .chunks(core::mem::size_of::<u16>())
            .all(|v| expected_write
                .as_slice()
                .chunks(core::mem::size_of::<u16>())
                .any(|u| u == v)));
    }
}
