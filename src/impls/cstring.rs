use alloc::borrow::Cow;
use alloc::ffi::CString;
use alloc::format;
use alloc::vec::Vec;
use no_std_io::io::{Read, Seek, Write};

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{ctx::*, DekuReader};
use crate::{DekuError, DekuWriter};

impl<Ctx: Copy> DekuWriter<Ctx> for CString
where
    u8: DekuWriter<Ctx>,
{
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        let bytes = self.as_bytes_with_nul();
        bytes.to_writer(writer, ctx)
    }
}

impl DekuReader<'_> for CString {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        _: (),
    ) -> Result<Self, DekuError> {
        let bytes =
            Vec::<u8>::from_reader_with_ctx(reader, (Limit::from(|b: &u8| *b == 0x00), ()))?;

        let value = CString::from_vec_with_nul(bytes).map_err(|e| {
            DekuError::Parse(Cow::from(format!("Failed to convert Vec to CString: {e}")))
        })?;

        Ok(value)
    }
}

impl<'a> DekuReader<'a, ByteSize> for CString
where
    u8: DekuReader<'a>,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        byte_size: ByteSize,
    ) -> Result<Self, DekuError> {
        let bytes = Vec::from_reader_with_ctx(reader, (Limit::from(byte_size.0), ()))?;

        let value = CString::from_vec_with_nul(bytes).map_err(|e| {
            DekuError::Parse(Cow::from(format!("Failed to convert Vec to CString: {e}")))
        })?;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use no_std_io::io::Cursor;
    use rstest::rstest;

    use crate::reader::Reader;

    use super::*;

    #[rstest(input, len, expected, expected_rest,
        case(
            b"test\0",
            Some(5),
            CString::new("test").unwrap(),
            &[],
        ),
        case(
            b"test\0",
            None,
            CString::new("test").unwrap(),
            &[],
        ),
        case(
            b"test\0a",
            Some(5),
            CString::new("test").unwrap(),
            b"a",
        ),
        case(
            b"test\0a",
            None,
            CString::new("test").unwrap(),
            b"a",
        ),

        #[should_panic(expected = "Parse(\"Failed to convert Vec to CString: data provided is not nul terminated\")")]
        case(
            b"test",
            Some(4),
            CString::new("test").unwrap(),
            b"a",
        ),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(b"test", Some(5), CString::new("test").unwrap(), &[]),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(b"test", None, CString::new("test").unwrap(), &[]),
    )]
    fn test_cstring_count(
        input: &[u8],
        len: Option<usize>,
        expected: CString,
        expected_rest: &[u8],
    ) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = if let Some(len) = len {
            CString::from_reader_with_ctx(&mut reader, ByteSize(len)).unwrap()
        } else {
            CString::from_reader_with_ctx(&mut reader, ()).unwrap()
        };
        assert_eq!(expected, res_read);
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest, buf);

        let mut writer = Writer::new(Cursor::new(vec![]));
        res_read.to_writer(&mut writer, ()).unwrap();
        assert_eq!(
            vec![b't', b'e', b's', b't', b'\0'],
            writer.inner.into_inner()
        );
    }
}
