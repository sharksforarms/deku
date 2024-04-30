use no_std_io::io::{Read, Write};

#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use alloc::format;

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

impl<'a, Ctx> DekuReader<'a, Ctx> for bool
where
    Ctx: Copy,
    u8: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<bool, DekuError> {
        let val = u8::from_reader_with_ctx(reader, inner_ctx)?;

        let ret = match val {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(DekuError::Parse(Cow::from(format!(
                "cannot parse bool value: {val}",
            )))),
        }?;

        Ok(ret)
    }
}

impl<Ctx> DekuWriter<Ctx> for bool
where
    u8: DekuWriter<Ctx>,
{
    /// wrapper around u8::write with consideration to context, such as bit size
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, inner_ctx: Ctx) -> Result<(), DekuError> {
        match self {
            true => (0x01u8).to_writer(writer, inner_ctx),
            false => (0x00u8).to_writer(writer, inner_ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use hexlit::hex;
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use crate::{ctx::BitSize, reader::Reader};

    use super::*;

    #[rstest(input, expected,
        case(&hex!("00"), false),
        case(&hex!("01"), true),

        #[should_panic(expected = "Parse(\"cannot parse bool value: 2\")")]
        case(&hex!("02"), false),
    )]
    fn test_bool(mut input: &[u8], expected: bool) {
        let mut reader = Reader::new(&mut input);
        let res_read = bool::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!(expected, res_read);
    }

    #[test]
    fn test_bool_with_context() {
        let input = &[0b01_000000];

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = bool::from_reader_with_ctx(&mut reader, crate::ctx::BitSize(2)).unwrap();
        assert!(res_read);
    }

    #[test]
    fn test_writer() {
        let mut writer = Writer::new(vec![]);
        true.to_writer(&mut writer, BitSize(1)).unwrap();
        assert_eq!(vec![true], writer.rest());

        let mut writer = Writer::new(vec![]);
        true.to_writer(&mut writer, ()).unwrap();
        assert_eq!(vec![1], writer.inner);

        let mut writer = Writer::new(vec![]);
        false.to_writer(&mut writer, ()).unwrap();
        assert_eq!(vec![0], writer.inner);
    }
}
