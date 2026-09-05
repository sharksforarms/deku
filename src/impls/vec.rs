use core::mem;

use no_std_io::io::{Read, Seek, SeekFrom, Write};

use alloc::vec::Vec;

use crate::error::NeedSize;
use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuWriter};
use crate::{DekuReader, ctx::*};

impl<'a> DekuReader<'a, ReadExact> for Vec<u8> {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        exact: ReadExact,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        // Check remaining bytes via the inner reader before allocating,
        // so a bogus count from untrusted input doesn't cause a huge
        // allocation that will immediately fail on read.
        let inner = reader.as_mut();
        let pos = inner
            .stream_position()
            .map_err(|e| DekuError::Io(e.kind()))?;
        let end = inner
            .seek(SeekFrom::End(0))
            .map_err(|e| DekuError::Io(e.kind()))?;
        inner
            .seek(SeekFrom::Start(pos))
            .map_err(|e| DekuError::Io(e.kind()))?;

        if (end - pos) < exact.0 as u64 {
            return Err(DekuError::Incomplete(NeedSize::new(exact.0 * 8)));
        }

        let mut bytes = alloc::vec![0x00; exact.0];
        let _ = reader.read_bytes(exact.0, &mut bytes, Order::Lsb0)?;
        Ok(bytes)
    }
}

impl<T> super::ReadCollection<T> for Vec<T> {
    fn with_capacity(capacity: Option<usize>) -> Self {
        capacity.map_or_else(Vec::new, Vec::with_capacity)
    }

    fn insert_item(&mut self, item: T) {
        self.push(item);
    }
}

impl<'a, T, Ctx, Predicate> DekuReader<'a, (Limit<T, Predicate>, Ctx)> for Vec<T>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(&T) -> bool,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        (limit, inner_ctx): (Limit<T, Predicate>, Ctx),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        if mem::size_of::<T>() == 0 {
            return Ok(Vec::new());
        }

        super::read_collection_with_limit(reader, limit, inner_ctx, |reader, ctx| {
            <T>::from_reader_with_ctx(reader, ctx)
        })
    }
}

impl<'a, T: DekuReader<'a>, Predicate: FnMut(&T) -> bool> DekuReader<'a, Limit<T, Predicate>>
    for Vec<T>
{
    /// Read `T`s until the given limit from input for types which don't require context.
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
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
    /// # #[cfg(feature = "bits")]
    /// # use deku::bitvec::{Msb0, bitvec};
    /// # #[cfg(feature = "std")]
    /// # use std::io::Cursor;
    ///
    /// # #[cfg(feature = "std")]
    /// # fn main() {
    /// let data = vec![1u8];
    /// let mut out_buf = vec![];
    /// let mut cursor = Cursor::new(&mut out_buf);
    /// let mut writer = Writer::new(&mut cursor);
    /// data.to_writer(&mut writer, Endian::Big).unwrap();
    /// assert_eq!(data, out_buf.to_vec());
    /// # }
    ///
    /// # #[cfg(not(feature = "std"))]
    /// # fn main() {}
    /// ```
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        for v in self {
            v.to_writer(writer, inner_ctx)?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    #[cfg(feature = "bits")]
    use crate::bitvec::{BitVec, Msb0, bits};
    use rstest::rstest;
    use std::io::Cursor;

    #[cfg(feature = "bits")]
    use crate::reader::Reader;

    use super::*;

    #[cfg(feature = "bits")]
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
        expected_rest_bits: BitVec<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut cursor = Cursor::new(&mut input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = Vec::<u8>::from_reader_with_ctx(&mut reader, limit).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[cfg(all(feature = "bits", feature = "descriptive-errors"))]
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
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u8, Predicate>,
        expected: Vec<u8>,
        expected_rest_bits: BitVec<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
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
            expected_rest_bits.iter().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[cfg(feature = "alloc")]
    #[rstest(input, endian, expected,
        case::normal(vec![0xAABB, 0xCCDD], Endian::Little, vec![0xBB, 0xAA, 0xDD, 0xCC]),
    )]
    fn test_vec_write(input: Vec<u16>, endian: Endian, expected: Vec<u8>) {
        let mut writer = Writer::new(Cursor::new(vec![]));
        input.to_writer(&mut writer, endian).unwrap();
        assert_eq!(expected, writer.inner.into_inner());
    }

    // Note: These tests also exist in boxed.rs
    #[cfg(feature = "bits")]
    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC], bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD], bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), vec![0xBBAA], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), vec![0xAABB], bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
    )]
    fn test_vec_reader_write<Predicate: FnMut(&u16) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: Vec<u16>,
        expected_rest_bits: BitVec<u8, Msb0>,
        expected_rest_bytes: &[u8],
        expected_write: Vec<u8>,
    ) {
        let input_clone = input;
        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read =
            Vec::<u16>::from_reader_with_ctx(&mut reader, (limit, (endian, BitSize(bit_size))))
                .unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read
            .to_writer(&mut writer, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, writer.inner.into_inner());

        assert_eq!(input_clone[..expected_write.len()].to_vec(), expected_write);
    }

    mod read_exact_tests {
        use super::*;
        use crate::DekuError;
        use crate::ctx::ReadExact;
        use crate::error::NeedSize;
        use crate::reader::Reader;
        use std::io::Cursor;

        #[test]
        fn read_exact_success() {
            let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
            let mut cursor = Cursor::new(data);
            let mut reader = Reader::new(&mut cursor);
            let result = Vec::<u8>::from_reader_with_ctx(&mut reader, ReadExact(4)).unwrap();
            assert_eq!(result, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        }

        #[test]
        fn read_exact_partial() {
            let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
            let mut cursor = Cursor::new(data);
            let mut reader = Reader::new(&mut cursor);
            let result = Vec::<u8>::from_reader_with_ctx(&mut reader, ReadExact(2)).unwrap();
            assert_eq!(result, vec![0xAA, 0xBB]);
            // remaining bytes still available
            let mut rest = vec![];
            cursor.read_to_end(&mut rest).unwrap();
            assert_eq!(rest, vec![0xCC, 0xDD]);
        }

        #[test]
        fn read_exact_not_enough_data() {
            let data = vec![0xAA, 0xBB];
            let mut cursor = Cursor::new(data);
            let mut reader = Reader::new(&mut cursor);
            let result = Vec::<u8>::from_reader_with_ctx(&mut reader, ReadExact(10));
            assert_eq!(
                result.unwrap_err(),
                DekuError::Incomplete(NeedSize::new(10 * 8))
            );
        }

        #[test]
        fn read_exact_empty_input() {
            let data: Vec<u8> = vec![];
            let mut cursor = Cursor::new(data);
            let mut reader = Reader::new(&mut cursor);
            let result = Vec::<u8>::from_reader_with_ctx(&mut reader, ReadExact(1));
            assert_eq!(result.unwrap_err(), DekuError::Incomplete(NeedSize::new(8)));
        }

        #[test]
        fn read_exact_zero_bytes() {
            let data = vec![0xAA];
            let mut cursor = Cursor::new(data);
            let mut reader = Reader::new(&mut cursor);
            let result = Vec::<u8>::from_reader_with_ctx(&mut reader, ReadExact(0)).unwrap();
            assert!(result.is_empty());
        }

        /// This test verifies the seek-based bounds check prevents
        /// a huge allocation when the requested count far exceeds
        /// available data. Without the fix, this would attempt to
        /// allocate ~1GB and zero-fill it before failing.
        #[test]
        fn read_exact_large_count_small_buffer_no_alloc() {
            let data = vec![0x01, 0x02, 0x03];
            let mut cursor = Cursor::new(data);
            let mut reader = Reader::new(&mut cursor);
            let result = Vec::<u8>::from_reader_with_ctx(&mut reader, ReadExact(1_073_741_824));
            assert_eq!(
                result.unwrap_err(),
                DekuError::Incomplete(NeedSize::new(1_073_741_824 * 8))
            );
        }
    }
}
