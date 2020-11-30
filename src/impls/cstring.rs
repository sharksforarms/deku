use crate::{ctx::*, DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;
use std::ffi::CString;

impl<Ctx: Copy> DekuWrite<Ctx> for CString
where
    u8: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<Msb0, u8>, ctx: Ctx) -> Result<(), DekuError> {
        let bytes = self.as_bytes_with_nul();
        bytes.write(output, ctx)?;

        Ok(())
    }
}

impl<Ctx: Copy> DekuRead<Ctx> for CString
where
    u8: DekuRead<Ctx>,
{
    fn read(input: &BitSlice<Msb0, u8>, ctx: Ctx) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, mut bytes) = Vec::read(input, (Limit::from(|b: &u8| *b == 0x00), ctx))?;

        // TODO: use from_vec_with_nul instead once stable

        // Remove null byte
        let nul_byte = bytes.pop();
        if nul_byte != Some(0x00) {
            return Err(DekuError::Unexpected("Expected nul byte".to_string()));
        }

        let value = CString::new(bytes).map_err(|e| {
            DekuError::Parse(format!(
                "Failed to convert Vec to CString: {}",
                e.to_string()
            ))
        })?;

        Ok((rest, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest(input, expected, expected_rest,
        case(
            &[b't', b'e', b's', b't', b'\0'],
            CString::new("test").unwrap(),
            bits![Msb0, u8;]
        ),
        case(
            &[b't', b'e', b's', b't', b'\0', b'a'],
            CString::new("test").unwrap(),
            [b'a'].view_bits::<Msb0>(),
        ),

        #[should_panic(expected = "Parse(\"not enough data: expected 8 bits got 0 bits\")")]
        case(&[b't', b'e', b's', b't'], CString::new("test").unwrap(), bits![Msb0, u8;]),
    )]
    fn test_cstring(input: &[u8], expected: CString, expected_rest: &BitSlice<Msb0, u8>) {
        let bit_slice = input.view_bits::<Msb0>();
        let (rest, res_read) = CString::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(vec![b't', b'e', b's', b't', b'\0'], res_write.into_vec());
    }
}
