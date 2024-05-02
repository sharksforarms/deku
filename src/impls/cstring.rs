use alloc::borrow::Cow;
use no_std_io::io::{Read, Write};
use std::ffi::CString;

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{ctx::*, DekuReader};
use crate::{DekuError, DekuWriter};

impl<Ctx: Copy> DekuWriter<Ctx> for CString
where
    u8: DekuWriter<Ctx>,
{
    fn to_writer<W: Write>(&self, writer: &mut Writer<W>, ctx: Ctx) -> Result<(), DekuError> {
        let bytes = self.as_bytes_with_nul();
        bytes.to_writer(writer, ctx)
    }
}

impl<'a, Ctx: Copy> DekuReader<'a, Ctx> for CString
where
    u8: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let bytes =
            Vec::from_reader_with_ctx(reader, (Limit::from(|b: &u8| *b == 0x00), inner_ctx))?;

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

    #[rstest(input, expected, expected_rest,
        case(
            &[b't', b'e', b's', b't', b'\0'],
            CString::new("test").unwrap(),
            &[],
        ),
        case(
            &[b't', b'e', b's', b't', b'\0', b'a'],
            CString::new("test").unwrap(),
            &[b'a'],
        ),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(&[b't', b'e', b's', b't'], CString::new("test").unwrap(), &[]),
    )]
    fn test_cstring(input: &[u8], expected: CString, expected_rest: &[u8]) {
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = CString::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!(expected, res_read);
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest, buf);

        let mut writer = Writer::new(vec![]);
        res_read.to_writer(&mut writer, ()).unwrap();
        assert_eq!(vec![b't', b'e', b's', b't', b'\0'], writer.inner);
    }
}
