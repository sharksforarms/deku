use alloc::sync::Arc;
use alloc::vec::Vec;
use no_std_io::io::{Read, Seek, Write};

use crate::ctx::Limit;
use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

impl<'a, T, Ctx> DekuReader<'a, Ctx> for Arc<T>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(Arc::new(val))
    }
}

impl<'a, T, Ctx, Predicate> DekuReader<'a, (Limit<T, Predicate>, Ctx)> for Arc<[T]>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(&T) -> bool,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        (limit, inner_ctx): (Limit<T, Predicate>, Ctx),
    ) -> Result<Self, DekuError> {
        // use Vec<T>'s implementation and convert to Arc<[T]>
        let val = <Vec<T>>::from_reader_with_ctx(reader, (limit, inner_ctx))?;
        Ok(Arc::from(val.into_boxed_slice()))
    }
}

impl<T, Ctx> DekuWriter<Ctx> for Arc<[T]>
where
    T: DekuWriter<Ctx>,
    Ctx: Copy,
{
    /// Write all `T`s to bits
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        for v in self.as_ref() {
            v.to_writer(writer, ctx)?;
        }
        Ok(())
    }
}

impl<T, Ctx> DekuWriter<Ctx> for Arc<T>
where
    T: DekuWriter<Ctx>,
    Ctx: Copy,
{
    /// Write all `T`s to bits
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        self.as_ref().to_writer(writer, ctx)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use super::*;
    use crate::ctx::*;
    use crate::native_endian;
    use crate::reader::Reader;
    #[cfg(feature = "bits")]
    use bitvec::prelude::*;

    #[rstest(input, expected,
        case(
            &[0xEF, 0xBE],
            Arc::new(native_endian!(0xBEEF_u16)),
        ),
    )]
    fn test_boxed(input: &[u8], expected: Arc<u16>) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = <Arc<u16>>::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!(expected, res_read);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read.to_writer(&mut writer, ()).unwrap();
        assert_eq!(input.to_vec(), writer.inner.into_inner());
    }

    // Note: Copied tests from vec.rs impl
    #[cfg(feature = "bits")]
    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), Arc::from(vec![0xBBAA, 0xDDCC].into_boxed_slice()), bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), Arc::from(vec![0xAABB, 0xCCDD].into_boxed_slice()), bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), Arc::from(vec![0xBBAA].into_boxed_slice()), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), Arc::from(vec![0xAABB].into_boxed_slice()), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), Arc::from(vec![0xBBAA].into_boxed_slice()), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), Arc::from(vec![0xAABB].into_boxed_slice()), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
    )]
    fn test_boxed_slice_from_reader_with_ctx<Predicate: FnMut(&u16) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: Arc<[u16]>,
        expected_rest_bits: &bitvec::slice::BitSlice<u8, bitvec::prelude::Msb0>,
        expected_rest_bytes: &[u8],
        expected_write: Vec<u8>,
    ) {
        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read =
            <Arc<[u16]>>::from_reader_with_ctx(&mut reader, (limit, (endian, BitSize(bit_size))))
                .unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read
            .to_writer(&mut writer, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, writer.inner.into_inner());

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }
}
