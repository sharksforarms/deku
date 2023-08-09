use acid_io::Read;
use std::ffi::CString;

use bitvec::prelude::*;

use crate::{ctx::*, DekuReader};
use crate::{DekuError, DekuRead, DekuWrite};

impl<Ctx: Copy> DekuWrite<Ctx> for CString
where
    u8: DekuWrite<Ctx>,
{
    fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
        let bytes = self.as_bytes_with_nul();
        bytes.write(output, ctx)
    }
}

impl<'a, Ctx: Copy> DekuRead<'a, Ctx> for CString
where
    u8: DekuRead<'a, Ctx>,
{
    fn read(input: &'a BitSlice<u8, Msb0>, ctx: Ctx) -> Result<(usize, Self), DekuError>
    where
        Self: Sized,
    {
        let (amt_read, mut bytes) = Vec::read(input, (Limit::from(|b: &u8| *b == 0x00), ctx))?;

        // TODO: use from_vec_with_nul instead once stable

        // Remove null byte
        let nul_byte = bytes.pop();
        if nul_byte != Some(0x00) {
            return Err(DekuError::Unexpected("Expected nul byte".to_string()));
        }

        let value = CString::new(bytes)
            .map_err(|e| DekuError::Parse(format!("Failed to convert Vec to CString: {e}")))?;

        Ok((amt_read, value))
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
    use rstest::rstest;

    use crate::container::Container;

    use super::*;

    #[rstest(input, expected, expected_rest,
        case(
            &[b't', b'e', b's', b't', b'\0'],
            CString::new("test").unwrap(),
            bits![u8, Msb0;]
        ),
        case(
            &[b't', b'e', b's', b't', b'\0', b'a'],
            CString::new("test").unwrap(),
            [b'a'].view_bits::<Msb0>(),
        ),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(&[b't', b'e', b's', b't'], CString::new("test").unwrap(), bits![u8, Msb0;]),
    )]
    fn test_cstring(input: &[u8], expected: CString, expected_rest: &BitSlice<u8, Msb0>) {
        let bit_slice = input.view_bits::<Msb0>();
        let (amt_read, res_read) = CString::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);

        let res_read = CString::from_reader(&mut Container::new(bit_slice), ()).unwrap();
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(vec![b't', b'e', b's', b't', b'\0'], res_write.into_vec());
    }
}
