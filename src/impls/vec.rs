use no_std_io::io::{Read, Write};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{ctx::*, DekuReader};
use crate::{DekuError, DekuWriter};

/// Read `T`s into a vec until a given predicate returns true
/// * `capacity` - an optional capacity to pre-allocate the vector with
/// * `ctx` - The context required by `T`. It will be passed to every `T` when constructing.
/// * `predicate` - the predicate that decides when to stop reading `T`s
/// The predicate takes two parameters: the number of bits that have been read so far,
/// and a borrow of the latest value to have been read. It should return `true` if reading
/// should now stop, and `false` otherwise
fn reader_vec_with_predicate<'a, T, Ctx, Predicate, R: Read>(
    reader: &mut Reader<R>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut predicate: Predicate,
) -> Result<Vec<T>, DekuError>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(usize, &T) -> bool,
{
    let mut res = capacity.map_or_else(Vec::new, Vec::with_capacity);

    let start_read = reader.bits_read;

    loop {
        let val = <T>::from_reader_with_ctx(reader, ctx)?;
        res.push(val);

        // This unwrap is safe as we are pushing to the vec immediately before it,
        // so there will always be a last element
        if predicate(reader.bits_read - start_read, res.last().unwrap()) {
            break;
        }
    }

    Ok(res)
}

fn reader_vec_to_end<'a, T, Ctx, R: Read>(
    reader: &mut crate::reader::Reader<R>,
    capacity: Option<usize>,
    ctx: Ctx,
) -> Result<Vec<T>, DekuError>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
{
    let mut res = capacity.map_or_else(Vec::new, Vec::with_capacity);
    loop {
        if reader.end() {
            break;
        }
        let val = <T>::from_reader_with_ctx(reader, ctx)?;
        res.push(val);
    }

    Ok(res)
}

impl<'a, T, Ctx, Predicate> DekuReader<'a, (Limit<T, Predicate>, Ctx)> for Vec<T>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(&T) -> bool,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        (limit, inner_ctx): (Limit<T, Predicate>, Ctx),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        match limit {
            // Read a given count of elements
            Limit::Count(mut count) => {
                // Handle the trivial case of reading an empty vector
                if count == 0 {
                    return Ok(Vec::new());
                }

                // Otherwise, read until we have read `count` elements
                reader_vec_with_predicate(reader, Some(count), inner_ctx, move |_, _| {
                    count -= 1;
                    count == 0
                })
            }

            // Read until a given predicate returns true
            Limit::Until(mut predicate, _) => {
                reader_vec_with_predicate(reader, None, inner_ctx, move |_, value| predicate(value))
            }

            // Read until a given quantity of bits have been read
            Limit::BitSize(size) => {
                let bit_size = size.0;

                // Handle the trivial case of reading an empty vector
                if bit_size == 0 {
                    return Ok(Vec::new());
                }

                reader_vec_with_predicate(reader, None, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }

            // Read until a given quantity of bytes have been read
            Limit::ByteSize(size) => {
                let bit_size = size.0 * 8;

                // Handle the trivial case of reading an empty vector
                if bit_size == 0 {
                    return Ok(Vec::new());
                }

                reader_vec_with_predicate(reader, None, inner_ctx, move |read_bits, _| {
                    read_bits == bit_size
                })
            }

            Limit::End => reader_vec_to_end(reader, None, inner_ctx),
        }
    }
}

impl<'a, T: DekuReader<'a>, Predicate: FnMut(&T) -> bool> DekuReader<'a, Limit<T, Predicate>>
    for Vec<T>
{
    /// Read `T`s until the given limit from input for types which don't require context.
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        limit: Limit<T, Predicate>,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        Vec::from_reader_with_ctx(reader, (limit, ()))
    }
}

impl<T: DekuWriter<Ctx>, Ctx: Copy> DekuWriter<Ctx> for Vec<T> {
    /// Write all `T`s in a `Vec` to bits.
    /// * **inner_ctx** - The context required by `T`.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWriter};
    /// # use deku::writer::Writer;
    /// # use deku::bitvec::{Msb0, bitvec};
    /// let data = vec![1u8];
    /// let mut out_buf = vec![];
    /// let mut writer = Writer::new(&mut out_buf);
    /// data.to_writer(&mut writer, Endian::Big).unwrap();
    /// assert_eq!(data, out_buf.to_vec());
    /// ```
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, inner_ctx: Ctx) -> Result<(), DekuError> {
        for v in self {
            v.to_writer(writer, inner_ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bitvec::{bits, BitSlice, Msb0};
    use rstest::rstest;

    use crate::reader::Reader;

    use super::*;

    #[rstest(input, limit, expected, expected_rest_bits, expected_rest_bytes,
        case::count_0([0xAA].as_ref(), 0.into(), vec![], bits![u8, Msb0;], &[0xaa]),
        case::count_1([0xAA, 0xBB].as_ref(), 1.into(), vec![0xAA], bits![u8, Msb0;], &[0xbb]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), 2.into(), vec![0xAA, 0xBB], bits![u8, Msb0;], &[0xcc]),
        case::until_null([0xAA, 0, 0xBB].as_ref(), (|v: &u8| *v == 0u8).into(), vec![0xAA, 0], bits![u8, Msb0;], &[0xbb]),
        case::until_bits([0xAA, 0xBB].as_ref(), BitSize(8).into(), vec![0xAA], bits![u8, Msb0;], &[0xbb]),
    )]
    fn test_vec_reader_no_ctx<Predicate: FnMut(&u8) -> bool>(
        mut input: &[u8],
        limit: Limit<u8, Predicate>,
        expected: Vec<u8>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut reader = Reader::new(&mut input);
        let res_read = Vec::<u8>::from_reader_with_ctx(&mut reader, limit).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        input.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes,
        case::count_0([0xAA].as_ref(), Endian::Little, Some(8), 0.into(), vec![], bits![u8, Msb0;], &[0xaa]),
        case::count_1([0xAA, 0xBB].as_ref(), Endian::Little, Some(8), 1.into(), vec![0xAA], bits![u8, Msb0;], &[0xbb]),
        case::count_2([0xAA, 0xBB, 0xCC].as_ref(), Endian::Little, Some(8), 2.into(), vec![0xAA, 0xBB], bits![u8, Msb0;], &[0xcc]),
        case::until_null([0xAA, 0, 0xBB].as_ref(), Endian::Little, None, (|v: &u8| *v == 0u8).into(), vec![0xAA, 0], bits![u8, Msb0;], &[0xbb]),
        case::until_bits([0xAA, 0xBB].as_ref(), Endian::Little, None, BitSize(8).into(), vec![0xAA], bits![u8, Msb0;], &[0xbb]),
        case::end([0xAA, 0xBB].as_ref(), Endian::Little, None, Limit::end(), vec![0xaa, 0xbb], bits![u8, Msb0;], &[]),
        case::end_bitsize([0xf0, 0xf0].as_ref(), Endian::Little, Some(4), Limit::end(), vec![0xf, 0x0, 0x0f, 0x0], bits![u8, Msb0;], &[]),
        case::bits_6([0b0110_1001, 0b1110_1001].as_ref(), Endian::Little, Some(6), 2.into(), vec![0b00_011010, 0b00_011110], bits![u8, Msb0; 1, 0, 0, 1], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(9), 1.into(), vec![], bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(9), 1.into(), vec![], bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data([0xAA].as_ref(), Endian::Little, Some(8), 2.into(), vec![], bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_until([0xAA].as_ref(), Endian::Little, Some(8), (|_: &u8| false).into(), vec![], bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case::not_enough_data_bits([0xAA].as_ref(), Endian::Little, Some(8), (BitSize(16)).into(), vec![], bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::too_much_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(9), 1.into(), vec![], bits![u8, Msb0;], &[]),
    )]
    fn test_vec_reader<Predicate: FnMut(&u8) -> bool>(
        mut input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u8, Predicate>,
        expected: Vec<u8>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut reader = Reader::new(&mut input);
        let res_read = match bit_size {
            Some(bit_size) => {
                Vec::<u8>::from_reader_with_ctx(&mut reader, (limit, (endian, BitSize(bit_size))))
                    .unwrap()
            }
            None => Vec::<u8>::from_reader_with_ctx(&mut reader, (limit, (endian))).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        input.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[rstest(input, endian, expected,
        case::normal(vec![0xAABB, 0xCCDD], Endian::Little, vec![0xBB, 0xAA, 0xDD, 0xCC]),
    )]
    fn test_vec_write(input: Vec<u16>, endian: Endian, expected: Vec<u8>) {
        let mut writer = Writer::new(vec![]);
        input.to_writer(&mut writer, endian).unwrap();
        assert_eq!(expected, writer.inner);
    }

    // Note: These tests also exist in boxed.rs
    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC], bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD], bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), vec![0xBBAA], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), vec![0xAABB], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
    )]
    fn test_vec_reader_write<Predicate: FnMut(&u16) -> bool>(
        mut input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: Vec<u16>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
        expected_write: Vec<u8>,
    ) {
        let input_clone = input;
        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let mut reader = Reader::new(&mut input);
        let res_read =
            Vec::<u16>::from_reader_with_ctx(&mut reader, (limit, (endian, BitSize(bit_size))))
                .unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        input.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);

        let mut writer = Writer::new(vec![]);
        res_read
            .to_writer(&mut writer, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, writer.inner);

        assert_eq!(input_clone[..expected_write.len()].to_vec(), expected_write);
    }
}
