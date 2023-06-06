use no_std_io::io::Read;

#[cfg(feature = "alloc")]
use alloc::format;

use bitvec::prelude::*;

use crate::{DekuError, DekuReader, DekuWrite};

impl<'a, Ctx> DekuReader<'a, Ctx> for bool
where
    Ctx: Copy,
    u8: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read>(
        reader: &mut crate::reader::Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<bool, DekuError> {
        let val = u8::from_reader_with_ctx(reader, inner_ctx)?;

        let ret = match val {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(DekuError::Parse(format!("cannot parse bool value: {val}",))),
        }?;

        Ok(ret)
    }
}

impl<Ctx> DekuWrite<Ctx> for bool
where
    u8: DekuWrite<Ctx>,
{
    /// wrapper around u8::write with consideration to context, such as bit size
    fn write(&self, output: &mut BitVec<u8, Msb0>, inner_ctx: Ctx) -> Result<(), DekuError> {
        match self {
            true => (0x01u8).write(output, inner_ctx),
            false => (0x00u8).write(output, inner_ctx),
        }
    }
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

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(vec![0b01], res_write.into_vec());
    }
}
