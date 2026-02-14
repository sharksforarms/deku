use no_std_io::io::{Read, Seek, Write};

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{deku_error, DekuError, DekuReader, DekuWriter};

impl<'a, Ctx> DekuReader<'a, Ctx> for bool
where
    Ctx: Clone,
    u8: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<bool, DekuError> {
        let val = u8::from_reader_with_ctx(reader, inner_ctx.clone())?;

        let ret = match val {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(deku_error!(
                DekuError::Parse,
                "cannot parse bool value",
                "{}",
                val
            )),
        }?;

        Ok(ret)
    }
}

impl<Ctx> DekuWriter<Ctx> for bool
where
    u8: DekuWriter<Ctx>,
{
    /// wrapper around u8::write with consideration to context, such as bit size
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        inner_ctx: Ctx,
    ) -> Result<(), DekuError> {
        match self {
            true => (0x01u8).to_writer(writer, inner_ctx),
            false => (0x00u8).to_writer(writer, inner_ctx),
        }
    }
}

impl crate::DekuSize for bool {
    const SIZE_BITS: usize = 8;
}

#[cfg(test)]
mod tests {
    use hexlit::hex;
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use crate::reader::Reader;

    use super::*;

    #[rstest(input, expected,
        case(&hex!("00"), false),
        case(&hex!("01"), true),

        #[should_panic(expected = "cannot parse bool value")]
        case(&hex!("02"), false),
    )]
    fn test_bool(input: &[u8], expected: bool) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = bool::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!(expected, res_read);
    }

    #[cfg(feature = "bits")]
    #[test]
    fn test_bool_with_context() {
        let input = &[0b01_000000];

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = bool::from_reader_with_ctx(&mut reader, crate::ctx::BitSize(2)).unwrap();
        assert!(res_read);
    }

    #[cfg(all(feature = "alloc", feature = "bits"))]
    #[test]
    fn test_writer_bits() {
        use crate::ctx::BitSize;

        let mut writer = Writer::new(Cursor::new(alloc::vec![]));
        true.to_writer(&mut writer, BitSize(1)).unwrap();
        assert_eq!(alloc::vec![true], writer.rest());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_writer() {
        let mut writer = Writer::new(Cursor::new(alloc::vec![]));
        true.to_writer(&mut writer, ()).unwrap();
        assert_eq!(alloc::vec![1], writer.inner.into_inner());

        let mut writer = Writer::new(Cursor::new(alloc::vec![]));
        false.to_writer(&mut writer, ()).unwrap();
        assert_eq!(alloc::vec![0], writer.inner.into_inner());
    }
}
