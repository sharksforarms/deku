use crate::{DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;

impl<Ctx: Copy> DekuRead<'_, Ctx> for () {
    /// NOP on read
    fn read(
        input: &BitSlice<Msb0, u8>,
        _inner_ctx: Ctx,
    ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        Ok((input, ()))
    }
}

impl<Ctx: Copy> DekuWrite<Ctx> for () {
    /// NOP on write
    fn write(&self, _output: &mut BitVec<Msb0, u8>, _inner_ctx: Ctx) -> Result<(), DekuError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexlit::hex;

    #[test]
    #[allow(clippy::unit_arg)]
    #[allow(clippy::unit_cmp)]
    fn test_unit() {
        let input = &hex!("FF");

        let bit_slice = input.view_bits::<Msb0>();
        let (rest, res_read) = <()>::read(bit_slice, ()).unwrap();
        assert_eq!((), res_read);
        assert_eq!(bit_slice, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(0, res_write.len());
    }
}
