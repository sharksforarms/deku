use no_std_io::io::{Read, Seek, Write};

use crate::{reader::Reader, writer::Writer, DekuError, DekuReader, DekuWriter};

impl<Ctx: Copy> DekuReader<'_, Ctx> for () {
    fn from_reader_with_ctx<R: Read + Seek>(
        _reader: &mut Reader<R>,
        _inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        Ok(())
    }
}

impl<Ctx: Copy> DekuWriter<Ctx> for () {
    /// NOP on write
    fn to_writer<W: Write + Seek>(
        &self,
        _writer: &mut Writer<W>,
        _inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        Ok(())
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use crate::reader::Reader;
    use std::io::Cursor;

    use super::*;

    #[test]
    #[allow(clippy::unit_arg)]
    #[allow(clippy::unit_cmp)]
    fn test_unit() {
        let input = &[0xff];

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        #[allow(clippy::let_unit_value)]
        let res_read = <()>::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!((), res_read);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read.to_writer(&mut writer, ()).unwrap();
        assert!(writer.inner.into_inner().is_empty());
    }
}
