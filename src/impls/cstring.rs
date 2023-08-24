use acid_io::Read;
use std::ffi::CString;

use bitvec::prelude::*;

use crate::{ctx::*, DekuReader};
use crate::{DekuError, DekuWrite};

impl<Ctx: Copy> DekuWrite<Ctx> for CString
where
    u8: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
        let bytes = self.as_bytes_with_nul();
        bytes.write(output, ctx)
    }
}

impl<'a, Ctx: Copy> DekuReader<'a, Ctx> for CString
where
    u8: DekuReader<'a, Ctx>,
{
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let bytes = Vec::from_reader(container, (Limit::from(|b: &u8| *b == 0x00), inner_ctx))?;

        let value = CString::from_vec_with_nul(bytes)
            .map_err(|e| DekuError::Parse(format!("Failed to convert Vec to CString: {e}")))?;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use acid_io::Cursor;
    use rstest::rstest;

    use crate::container::Container;

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
        let mut container = Container::new(&mut cursor);
        let res_read = CString::from_reader(&mut container, ()).unwrap();
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest, buf);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(vec![b't', b'e', b's', b't', b'\0'], res_write.into_vec());
    }
}
