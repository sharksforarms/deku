use core::hash::{BuildHasher, Hash};
use std::collections::HashMap;

use no_std_io::io::{Read, Seek, Write};

use crate::ctx::*;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

/// Read `K, V`s into a hashmap until a given predicate returns true
/// * `capacity` - an optional capacity to pre-allocate the hashmap with
/// * `ctx` - The context required by `K, V`. It will be passed to every `K, V` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `K, V`s
///   The predicate takes two parameters: the number of bits that have been read so far,
///   and a borrow of the latest value to have been read. It should return `true` if reading
///   should now stop, and `false` otherwise
#[allow(clippy::type_complexity)]
fn from_reader_with_ctx_hashmap_with_predicate<'a, K, V, S, Ctx, Predicate, R: Read + Seek>(
    reader: &mut crate::reader::Reader<R>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<HashMap<K, V, S>, DekuError>
where
    K: DekuReader<'a, Ctx> + Eq + Hash,
    V: DekuReader<'a, Ctx>,
    S: BuildHasher + Default,
    Ctx: Copy,
    Predicate: FnMut(usize, &(K, V)) -> bool,
{
    let mut res = HashMap::with_capacity_and_hasher(capacity.unwrap_or(0), S::default());

    let mut found_predicate = false;
    let orig_bits_read = reader.bits_read;

    while !found_predicate {
        let val = <(K, V)>::from_reader_with_ctx(reader, ctx)?;
        found_predicate = predicate(reader.bits_read - orig_bits_read, &val);
        res.insert(val.0, val.1);
    }

    Ok(res)
}

fn from_reader_with_ctx_hashmap_to_end<'a, K, V, S, Ctx, R: Read + Seek>(
    reader: &mut crate::reader::Reader<R>,
    capacity: Option<usize>,
    ctx: Ctx,
) -> Result<HashMap<K, V, S>, DekuError>
where
    K: DekuReader<'a, Ctx> + Eq + Hash,
    V: DekuReader<'a, Ctx>,
    S: BuildHasher + Default,
    Ctx: Copy,
{
    let mut res = HashMap::with_capacity_and_hasher(capacity.unwrap_or(0), S::default());

    loop {
        if reader.end() {
            break;
        }
        let val = <(K, V)>::from_reader_with_ctx(reader, ctx)?;
        res.insert(val.0, val.1);
    }

    Ok(res)
}

impl<'a, K, V, S, Ctx, Predicate> DekuReader<'a, (Limit<(K, V), Predicate>, Ctx)>
    for HashMap<K, V, S>
where
    K: DekuReader<'a, Ctx> + Eq + Hash,
    V: DekuReader<'a, Ctx>,
    S: BuildHasher + Default,
    Ctx: Copy,
    Predicate: FnMut(&(K, V)) -> bool,
{
    /// Read `T`s until the given limit
    /// * `limit` - the limiting factor on the amount of `T`s to read
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuReader;
    /// # #[cfg(feature = "std")]
    /// # use std::collections::HashMap;
    /// # #[cfg(feature = "std")]
    /// # use std::io::Cursor;
    ///
    /// # #[cfg(feature = "std")]
    /// # fn main() {
    /// let mut input = Cursor::new(vec![100, 1, 2, 3, 4]);
    /// let mut reader = deku::reader::Reader::new(&mut input);
    /// let map =
    ///     HashMap::<u8, u32>::from_reader_with_ctx(&mut reader, (1.into(), Endian::Little)).unwrap();
    /// let mut expected = HashMap::<u8, u32>::default();
    /// expected.insert(100, 0x04030201);
    /// assert_eq!(expected, map)
    /// # }
    ///
    /// # #[cfg(not(feature = "std"))]
    /// # fn main() {}
    /// ```
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut crate::reader::Reader<R>,
        (limit, inner_ctx): (Limit<(K, V), Predicate>, Ctx),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        match limit {
            // Read a given count of elements
            Limit::Count(mut count) => {
                // Handle the trivial case of reading an empty hashmap
                if count == 0 {
                    return Ok(HashMap::<K, V, S>::default());
                }

                // Otherwise, read until we have read `count` elements
                from_reader_with_ctx_hashmap_with_predicate(
                    reader,
                    Some(count),
                    inner_ctx,
                    move |_, _| {
                        count -= 1;
                        count == 0
                    },
                )
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => from_reader_with_ctx_hashmap_with_predicate(
                reader,
                None,
                inner_ctx,
                move |_, kv| predicate(kv),
            ),

            // Read until a given quantity of bits have been read
            Limit::BitSize(size) => {
                let bit_size = size.0;

                // Handle the trivial case of reading an empty hashmap
                if bit_size == 0 {
                    return Ok(HashMap::<K, V, S>::default());
                }

                from_reader_with_ctx_hashmap_with_predicate(
                    reader,
                    None,
                    inner_ctx,
                    move |read_bits, _| read_bits == bit_size,
                )
            }

            // Read until a given quantity of byte bits have been read
            Limit::ByteSize(size) => {
                let bit_size = size.0 * 8;

                // Handle the trivial case of reading an empty hashmap
                if bit_size == 0 {
                    return Ok(HashMap::<K, V, S>::default());
                }

                from_reader_with_ctx_hashmap_with_predicate(
                    reader,
                    None,
                    inner_ctx,
                    move |read_bits, _| read_bits == bit_size,
                )
            }

            // Read until `reader.end()` is true
            Limit::End => from_reader_with_ctx_hashmap_to_end(reader, None, inner_ctx),
        }
    }
}

impl<'a, K, V, S, Predicate> DekuReader<'a, Limit<(K, V), Predicate>> for HashMap<K, V, S>
where
    K: DekuReader<'a> + Eq + Hash,
    V: DekuReader<'a>,
    S: BuildHasher + Default,
    Predicate: FnMut(&(K, V)) -> bool,
{
    /// Read `K, V`s until the given limit from input for types which don't require context.
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut crate::reader::Reader<R>,
        limit: Limit<(K, V), Predicate>,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        Self::from_reader_with_ctx(reader, (limit, ()))
    }
}

impl<K: DekuWriter<Ctx>, V: DekuWriter<Ctx>, S, Ctx: Copy> DekuWriter<Ctx> for HashMap<K, V, S> {
    /// Write all `K, V`s in a `HashMap` to bits.
    /// * **inner_ctx** - The context required by `K, V`.
    ///
    /// Note: depending on the Hasher `S`, the order in which the `K, V` pairs are
    /// written may change between executions. Use a deterministic Hasher for your HashMap
    /// instead of the default RandomState hasher if you don't want the order written to change.
    ///
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWriter};
    /// # use deku::writer::Writer;
    /// # #[cfg(feature = "bits")]
    /// # use deku::bitvec::{Msb0, bitvec};
    /// # #[cfg(feature = "std")]
    /// # use std::collections::HashMap;
    /// # #[cfg(feature = "std")]
    /// # use std::io::Cursor;
    ///
    /// # #[cfg(feature = "std")]
    /// # fn main() {
    /// let mut out_buf = vec![];
    /// let mut cursor = Cursor::new(&mut out_buf);
    /// let mut writer = Writer::new(&mut cursor);
    /// let mut map = HashMap::<u8, u32>::default();
    /// map.insert(100, 0x04030201);
    /// map.to_writer(&mut writer, Endian::Big).unwrap();
    /// let expected: Vec<u8> = vec![100, 4, 3, 2, 1];
    /// assert_eq!(expected, out_buf);
    /// # }
    ///
    /// # #[cfg(not(feature = "std"))]
    /// fn main() {}
    /// ```
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        for kv in self {
            kv.to_writer(writer, inner_ctx)?;
        }
        Ok(())
    }
}

#[cfg(all(feature = "bits", feature = "descriptive-errors"))]
#[cfg(test)]
mod tests {
    use no_std_io::io::Cursor;
    use rstest::rstest;
    use rustc_hash::FxHashMap;

    use crate::reader::Reader;

    use super::*;
    use bitvec::prelude::*;

    // Macro to create a deterministic HashMap for tests
    // This is needed for tests since the default HashMap Hasher
    // RandomState will Hash the keys different for each run of the test cases
    // and will make it harder to compare the output of DekuWrite for HashMaps
    // with multiple K, V pairs
    macro_rules! fxhashmap(
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = FxHashMap::default();
                $(
                    m.insert($key, $value);
                )+
                m
            }
         };
    );

    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes,
        case::count_0([0xAA].as_ref(), Endian::Little, Some(8), 0.into(), FxHashMap::default(), bits![u8, Msb0;], &[0xaa]),
        case::count_1([0x01, 0xAA, 0x02, 0xBB].as_ref(), Endian::Little, Some(8), 1.into(), fxhashmap!{0x01 => 0xAA}, bits![u8, Msb0;], &[0x02, 0xbb]),
        case::count_2([0x01, 0xAA, 0x02, 0xBB, 0xBB].as_ref(), Endian::Little, Some(8), 2.into(), fxhashmap!{0x01 => 0xAA, 0x02 => 0xBB}, bits![u8, Msb0;], &[0xbb]),
        case::until_null([0x01, 0xAA, 0, 0, 0xBB].as_ref(), Endian::Little, None, (|kv: &(u8, u8)| kv.0 == 0u8 && kv.1 == 0u8).into(), fxhashmap!{0x01 => 0xAA, 0 => 0}, bits![u8, Msb0;], &[0xbb]),
        case::until_empty_bits([0x01, 0xAA, 0xBB].as_ref(), Endian::Little, None, BitSize(0).into(), FxHashMap::default(), bits![u8, Msb0;], &[0x01, 0xaa, 0xbb]),
        case::until_empty_bytes([0x01, 0xAA, 0xBB].as_ref(), Endian::Little, None, ByteSize(0).into(), FxHashMap::default(), bits![u8, Msb0;], &[0x01, 0xaa, 0xbb]),
        case::until_bits([0x01, 0xAA, 0xBB].as_ref(), Endian::Little, None, BitSize(16).into(), fxhashmap!{0x01 => 0xAA}, bits![u8, Msb0;], &[0xbb]),
        case::read_all([0x01, 0xAA].as_ref(), Endian::Little, None, Limit::end(), fxhashmap!{0x01 => 0xAA}, bits![u8, Msb0;], &[]),
        case::until_bytes([0x01, 0xAA, 0xBB].as_ref(), Endian::Little, None, ByteSize(2).into(), fxhashmap!{0x01 => 0xAA}, bits![u8, Msb0;], &[0xbb]),
        case::until_count([0x01, 0xAA, 0xBB].as_ref(), Endian::Little, None, Limit::from(1), fxhashmap!{0x01 => 0xAA}, bits![u8, Msb0;], &[0xbb]),
        case::bits_6([0b0000_0100, 0b1111_0000, 0b1000_0000].as_ref(), Endian::Little, Some(6), 2.into(), fxhashmap!{0x01 => 0x0F, 0x02 => 0}, bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(9), 1.into(), FxHashMap::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(9), 1.into(), FxHashMap::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(8), 2.into(), FxHashMap::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_until([0xAA].as_ref(), Endian::Little, Some(8), (|_: &(u8, u8)| false).into(), FxHashMap::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_bits([0xAA].as_ref(), Endian::Little, Some(8), (BitSize(16)).into(), FxHashMap::default(), bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(9), 1.into(), FxHashMap::default(), bits![u8, Msb0;], &[]),
    )]
    fn test_hashmap_read<Predicate: FnMut(&(u8, u8)) -> bool + Copy>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<(u8, u8), Predicate>,
        expected: FxHashMap<u8, u8>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = match bit_size {
            Some(bit_size) => FxHashMap::<u8, u8>::from_reader_with_ctx(
                &mut reader,
                (limit, (endian, BitSize(bit_size))),
            )
            .unwrap(),
            None => {
                FxHashMap::<u8, u8>::from_reader_with_ctx(&mut reader, (limit, (endian))).unwrap()
            }
        };
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
        case::normal(fxhashmap!{0x11u8 => 0xAABBu16, 0x23u8 => 0xCCDDu16}, Endian::Little, vec![0x23, 0xDD, 0xCC, 0x11, 0xBB, 0xAA]),
    )]
    fn test_hashmap_write(input: FxHashMap<u8, u16>, endian: Endian, expected: Vec<u8>) {
        let mut writer = Writer::new(Cursor::new(vec![]));
        input.to_writer(&mut writer, endian).unwrap();
        assert_eq!(expected, writer.inner.into_inner());
    }
}
