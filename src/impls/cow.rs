use alloc::borrow::{Borrow, Cow};

use no_std_io::io::{Read, Seek, Write};

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

impl<'a, T, Ctx> DekuReader<'a, Ctx> for Cow<'a, T>
where
    T: DekuReader<'a, Ctx> + Clone,
    Ctx: Copy,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(Cow::Owned(val))
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

#[cfg(test)]
mod tests {
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use super::*;
    use crate::{native_endian, reader::Reader};

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
