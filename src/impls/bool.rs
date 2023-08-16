use acid_io::Read;

#[cfg(feature = "alloc")]
use alloc::format;

use bitvec::prelude::*;

use crate::{DekuError, DekuRead, DekuReader, DekuWrite};

impl<'a, Ctx> DekuRead<'a, Ctx> for bool
where
    Ctx: Copy,
    u8: DekuRead<'a, Ctx>,
{
    /// wrapper around u8::read with consideration to context, such as bit size
    /// true if the result of the read is `1`, false if `0` and error otherwise
    fn read(input: &'a BitSlice<u8, Msb0>, inner_ctx: Ctx) -> Result<(usize, Self), DekuError> {
        let (amt_read, val) = u8::read(input, inner_ctx)?;

        let ret = match val {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(DekuError::Parse(format!("cannot parse bool value: {val}",))),
        }?;

        Ok((amt_read, ret))
    }
}

impl<'a, Ctx> DekuReader<'a, Ctx> for bool
where
    Ctx: Copy,
    u8: DekuReader<'a, Ctx>,
{
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        inner_ctx: Ctx,
    ) -> Result<bool, DekuError> {
        let val = u8::from_reader(container, inner_ctx)?;

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
    use acid_io::Cursor;
    use hexlit::hex;
    use rstest::rstest;

    use crate::container::Container;

    use super::*;

    #[rstest(input, expected,
        case(&hex!("00"), false),
        case(&hex!("01"), true),

        #[should_panic(expected = "Parse(\"cannot parse bool value: 2\")")]
        case(&hex!("02"), false),
    )]
    fn test_bool(mut input: &[u8], expected: bool) {
        let mut container = Container::new(&mut input);
        let res_read = bool::from_reader(&mut container, ()).unwrap();
        assert_eq!(expected, res_read);
    }

    #[rstest(input, expected,
        case(&hex!("00"), false),
        case(&hex!("01"), true),

        #[should_panic(expected = "Parse(\"cannot parse bool value: 2\")")]
        case(&hex!("02"), false),
    )]
    fn test_bool_read(mut input: &[u8], expected: bool) {
        let mut container = Container::new(&mut input);
        let bit_slice = input.view_bits::<Msb0>();
        let (amt_read, res_read) = bool::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(amt_read, 8);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[test]
    fn test_bool_with_context() {
        let input = &[0b01_000000];
        let bit_slice = input.view_bits::<Msb0>();

        let (amt_read, res_read) = bool::read(bit_slice, crate::ctx::BitSize(2)).unwrap();
        assert!(res_read);
        assert_eq!(amt_read, 2);

        let mut cursor = Cursor::new(input);
        let mut container = Container::new(&mut cursor);
        let res_read = bool::from_reader(&mut container, crate::ctx::BitSize(2)).unwrap();
        assert!(res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(vec![0b01], res_write.into_vec());
    }
}
