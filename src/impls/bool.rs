use crate::{DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;

#[cfg(feature = "alloc")]
use alloc::format;

impl<'a, Ctx> DekuRead<'a, Ctx> for bool
where
    Ctx: Copy,
    u8: DekuRead<'a, Ctx>,
{
    /// wrapper around u8::read with consideration to context, such as bit size
    /// true if the result of the read is `1`, false if `0` and error otherwise
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        inner_ctx: Ctx,
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError> {
        let (rest, val) = u8::read(input, inner_ctx)?;

        let ret = match val {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(DekuError::Parse(format!(
                "cannot parse bool value: {}",
                val
            ))),
        }?;

        Ok((rest, ret))
    }
}

impl<Ctx> DekuWrite<Ctx> for bool
where
    u8: DekuWrite<Ctx>,
{
    /// wrapper around u8::write with consideration to context, such as bit size
    fn write(&self, output: &mut BitVec<Msb0, u8>, inner_ctx: Ctx) -> Result<(), DekuError> {
        match self {
            true => (0x01u8).write(output, inner_ctx),
            false => (0x00u8).write(output, inner_ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;
    use rstest::rstest;

    #[rstest(input, expected,
        case(&hex!("00"), false),
        case(&hex!("01"), true),

        #[should_panic(expected = "Parse(\"cannot parse bool value: 2\")")]
        case(&hex!("02"), false),
    )]
    fn test_bool(input: &[u8], expected: bool) {
        let bit_slice = input.view_bits::<Msb0>();
        let (rest, res_read) = bool::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert!(rest.is_empty());

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    #[test]
    fn test_bool_with_context() {
        let input = &[0b01_000000];
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = bool::read(bit_slice, crate::ctx::Size::Bits(2)).unwrap();
        assert_eq!(true, res_read);
        assert_eq!(6, rest.len());

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(vec![0b01], res_write.into_vec());
    }
}
