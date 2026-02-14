use no_std_io::io::{Read, Seek, Write};

use crate::{writer::Writer, DekuError, DekuReader, DekuWriter};

impl<'a, T: DekuReader<'a, Ctx>, Ctx: Clone> DekuReader<'a, Ctx> for Option<T> {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut crate::reader::Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader_with_ctx(reader, inner_ctx.clone())?;
        Ok(Some(val))
    }
}

impl<T: DekuWriter<Ctx>, Ctx: Clone> DekuWriter<Ctx> for Option<T> {
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        self.as_ref()
            .map_or(Ok(()), |v| v.to_writer(writer, inner_ctx.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use no_std_io::io::Cursor;

    use crate::reader::Reader;

    #[test]
    fn test_option_read() {
        use crate::ctx::*;
        let input = &[1u8, 2, 3, 4];
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let v = Option::<u32>::from_reader_with_ctx(&mut reader, Endian::Little).unwrap();
        assert_eq!(v, Some(0x04030201))
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_option_write() {
        let mut writer = Writer::new(Cursor::new(alloc::vec![]));
        Some(true).to_writer(&mut writer, ()).unwrap();
        assert_eq!(alloc::vec![1], writer.inner.into_inner());
    }
}
