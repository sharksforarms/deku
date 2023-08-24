use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};

use acid_io::Read;
use bitvec::prelude::*;

use crate::ctx::*;
use crate::{DekuError, DekuReader, DekuWrite};

/// Read `K, V`s into a hashmap until a given predicate returns true
/// * `capacity` - an optional capacity to pre-allocate the hashmap with
/// * `ctx` - The context required by `K, V`. It will be passed to every `K, V` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `K, V`s
/// The predicate takes two parameters: the number of bits that have been read so far,
/// and a borrow of the latest value to have been read. It should return `true` if reading
/// should now stop, and `false` otherwise
#[allow(clippy::type_complexity)]
fn from_reader_hashmap_with_predicate<'a, K, V, S, Ctx, Predicate, R: Read>(
    container: &mut crate::container::Container<R>,
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
    let orig_bits_read = container.bits_read;

    while !found_predicate {
        let val = <(K, V)>::from_reader(container, ctx)?;
        found_predicate = predicate(container.bits_read - orig_bits_read, &val);
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
    /// # use std::collections::HashMap;
    /// # use std::io::Cursor;
    /// let mut input = Cursor::new(vec![100, 1, 2, 3, 4]);
    /// let mut container = deku::container::Container::new(&mut input);
    /// let map =
    ///     HashMap::<u8, u32>::from_reader(&mut container, (1.into(), Endian::Little)).unwrap();
    /// let mut expected = HashMap::<u8, u32>::default();
    /// expected.insert(100, 0x04030201);
    /// assert_eq!(expected, map)
    /// ```
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
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
                from_reader_hashmap_with_predicate(
                    container,
                    Some(count),
                    inner_ctx,
                    move |_, _| {
                        count -= 1;
                        count == 0
                    },
                )
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => {
                from_reader_hashmap_with_predicate(container, None, inner_ctx, move |_, kv| {
                    predicate(kv)
                })
            }

            // Read until a given quantity of bits have been read
            Limit::BitSize(size) => {
                let bit_size = size.0;
                from_reader_hashmap_with_predicate(
                    container,
                    None,
                    inner_ctx,
                    move |read_bits, _| read_bits == bit_size,
                )
            }

            // Read until a given quantity of byte bits have been read
            Limit::ByteSize(size) => {
                let bit_size = size.0 * 8;
                from_reader_hashmap_with_predicate(
                    container,
                    None,
                    inner_ctx,
                    move |read_bits, _| read_bits == bit_size,
                )
            }
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
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        limit: Limit<(K, V), Predicate>,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        Self::from_reader(container, (limit, ()))
    }
}

impl<K: DekuWrite<Ctx>, V: DekuWrite<Ctx>, S, Ctx: Copy> DekuWrite<Ctx> for HashMap<K, V, S> {
    /// Write all `K, V`s in a `HashMap` to bits.
    /// * **inner_ctx** - The context required by `K, V`.
    /// Note: depending on the Hasher `S`, the order in which the `K, V` pairs are
    /// written may change between executions. Use a deterministic Hasher for your HashMap
    /// instead of the default RandomState hasher if you don't want the order written to change.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite};
    /// # use deku::bitvec::{Msb0, bitvec};
    /// # use std::collections::HashMap;
    /// let mut output = bitvec![u8, Msb0;];
    /// let mut map = HashMap::<u8, u32>::default();
    /// map.insert(100, 0x04030201);
    /// map.write(&mut output, Endian::Big).unwrap();
    /// let expected: Vec<u8> = vec![100, 4, 3, 2, 1];
    /// assert_eq!(expected, output.into_vec())
    /// ```
    fn write(&self, output: &mut BitVec<u8, Msb0>, inner_ctx: Ctx) -> Result<(), DekuError> {
        for kv in self {
            kv.write(output, inner_ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use acid_io::Cursor;
    use rstest::rstest;
    use rustc_hash::FxHashMap;

    use crate::container::Container;

    use super::*;

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
        case::until_bits([0x01, 0xAA, 0xBB].as_ref(), Endian::Little, None, BitSize(16).into(), fxhashmap!{0x01 => 0xAA}, bits![u8, Msb0;], &[0xbb]),
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
        let mut container = Container::new(&mut cursor);
        let res_read = match bit_size {
            Some(bit_size) => FxHashMap::<u8, u8>::from_reader(
                &mut container,
                (limit, (endian, BitSize(bit_size))),
            )
            .unwrap(),
            None => FxHashMap::<u8, u8>::from_reader(&mut container, (limit, (endian))).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(
            container.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[rstest(input, endian, expected,
        case::normal(fxhashmap!{0x11u8 => 0xAABBu16, 0x23u8 => 0xCCDDu16}, Endian::Little, vec![0x11, 0xBB, 0xAA, 0x23, 0xDD, 0xCC]),
    )]
    fn test_hashmap_write(input: FxHashMap<u8, u16>, endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }

    // Note: These tests also exist in boxed.rs
    #[rstest(input, endian, limit, expected, expected_rest_bits, expected_rest_bytes, expected_write,
        case::normal_le([0xAA, 0xBB, 0, 0xCC, 0xDD, 0].as_ref(), Endian::Little, 2.into(), fxhashmap!{0xBBAA => 0, 0xDDCC => 0}, bits![u8, Msb0;], &[], vec![0xCC, 0xDD, 0, 0xAA, 0xBB, 0]),
        case::normal_be([0xAA, 0xBB, 0, 0xCC, 0xDD, 0].as_ref(), Endian::Big, 2.into(), fxhashmap!{0xAABB => 0, 0xCCDD => 0}, bits![u8, Msb0;], &[], vec![0xCC, 0xDD, 0, 0xAA, 0xBB, 0]),
        case::predicate_le([0xAA, 0xBB, 0, 0xCC, 0xDD, 0].as_ref(), Endian::Little, (|kv: &(u16, u8)| kv.0 == 0xBBAA && kv.1 == 0).into(), fxhashmap!{0xBBAA => 0}, bits![u8, Msb0;], &[0xcc, 0xdd, 0], vec![0xAA, 0xBB, 0]),
        case::predicate_be([0xAA, 0xBB, 0, 0xCC, 0xDD, 0].as_ref(), Endian::Big, (|kv: &(u16, u8)| kv.0 == 0xAABB && kv.1 == 0).into(), fxhashmap!{0xAABB => 0}, bits![u8, Msb0;], &[0xcc, 0xdd, 0], vec![0xAA, 0xBB, 0]),
        case::bytes_le([0xAA, 0xBB, 0, 0xCC, 0xDD, 0].as_ref(), Endian::Little, BitSize(24).into(), fxhashmap!{0xBBAA => 0}, bits![u8, Msb0;], &[0xcc, 0xdd, 0], vec![0xAA, 0xBB, 0]),
        case::bytes_be([0xAA, 0xBB, 0, 0xCC, 0xDD, 0].as_ref(), Endian::Big, BitSize(24).into(), fxhashmap!{0xAABB => 0}, bits![u8, Msb0;], &[0xcc, 0xdd, 0], vec![0xAA, 0xBB, 0]),
    )]
    fn test_hashmap_read_write<Predicate: FnMut(&(u16, u8)) -> bool + Copy>(
        input: &[u8],
        endian: Endian,
        limit: Limit<(u16, u8), Predicate>,
        expected: FxHashMap<u16, u8>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
        expected_write: Vec<u8>,
    ) {
        let mut cursor = Cursor::new(input);
        let mut container = Container::new(&mut cursor);
        let res_read = FxHashMap::<u16, u8>::from_reader(&mut container, (limit, endian)).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            container.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, endian).unwrap();
        assert_eq!(expected_write, res_write.into_vec());
    }
}
