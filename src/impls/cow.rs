use alloc::borrow::{Borrow, Cow};
use alloc::vec::Vec;

use no_std_io::io::{Read, Seek, Write};

use crate::ctx::{BitSize, ByteSize, Limit, ReadExact};
use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

impl<'a, T, Ctx> DekuReader<'a, Ctx> for Cow<'a, T>
where
    T: DekuReader<'a, Ctx> + Clone,
    Ctx: Copy,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(Cow::Owned(val))
    }
}

impl<'a> DekuReader<'a, ReadExact> for Cow<'a, [u8]> {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        exact: ReadExact,
    ) -> Result<Self, DekuError> {
        if reader.source_bytes().is_some() {
            <&'a [u8]>::from_reader_with_ctx(reader, exact).map(Cow::Borrowed)
        } else {
            Vec::<u8>::from_reader_with_ctx(reader, exact).map(Cow::Owned)
        }
    }
}

impl<'a, Ctx, Predicate> DekuReader<'a, (Limit<u8, Predicate>, Ctx)> for Cow<'a, [u8]>
where
    Predicate: FnMut(&u8) -> bool,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        (limit, _inner_ctx): (Limit<u8, Predicate>, Ctx),
    ) -> Result<Self, DekuError> {
        if reader.source_bytes().is_some() {
            <&'a [u8]>::from_reader_with_ctx(reader, (limit, ())).map(Cow::Borrowed)
        } else {
            Vec::<u8>::from_reader_with_ctx(reader, (limit, ())).map(Cow::Owned)
        }
    }
}

impl<'a, Predicate> DekuReader<'a, Limit<u8, Predicate>> for Cow<'a, [u8]>
where
    Predicate: FnMut(&u8) -> bool,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        limit: Limit<u8, Predicate>,
    ) -> Result<Self, DekuError> {
        Self::from_reader_with_ctx(reader, (limit, ()))
    }
}

impl<'a> DekuReader<'a, ByteSize> for Cow<'a, [u8]> {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        size: ByteSize,
    ) -> Result<Self, DekuError> {
        if reader.source_bytes().is_some() {
            <&'a [u8]>::from_reader_with_ctx(reader, size).map(Cow::Borrowed)
        } else {
            Vec::<u8>::from_reader_with_ctx(reader, ReadExact(size.0)).map(Cow::Owned)
        }
    }
}

impl<'a> DekuReader<'a, BitSize> for Cow<'a, [u8]> {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        size: BitSize,
    ) -> Result<Self, DekuError> {
        if reader.source_bytes().is_some() {
            <&'a [u8]>::from_reader_with_ctx(reader, size).map(Cow::Borrowed)
        } else {
            Vec::<u8>::from_reader_with_ctx(reader, (Limit::from(size), ())).map(Cow::Owned)
        }
    }
}

impl<T, Ctx> DekuWriter<Ctx> for Cow<'_, T>
where
    T: DekuWriter<Ctx> + Clone,
    Ctx: Copy,
{
    /// Write T from Cow<T>
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        (self.borrow() as &T).to_writer(writer, inner_ctx)
    }
}

impl<Ctx: Copy> DekuWriter<Ctx> for Cow<'_, [u8]>
where
    u8: DekuWriter<Ctx>,
{
    /// Write bytes from a borrowed or owned `Cow<[u8]>`.
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        self.as_ref().to_writer(writer, inner_ctx)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use alloc::vec;
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use super::*;
    use crate::{native_endian, reader::Reader};

    #[cfg(feature = "alloc")]
    #[rstest(input, expected,
        case(
            &[0xEF, 0xBE],
            Cow::Owned(native_endian!(0xBEEF_u16)),
        ),
    )]
    fn test_cow(input: &[u8], expected: Cow<u16>) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = <Cow<u16>>::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!(expected, res_read);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read.to_writer(&mut writer, ()).unwrap();
        assert_eq!(input.to_vec(), writer.inner.into_inner());
    }
}
