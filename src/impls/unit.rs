use acid_io::Read;
use bitvec::prelude::*;

use crate::{DekuError, DekuRead, DekuReader, DekuWrite};

impl<Ctx: Copy> DekuRead<'_, Ctx> for () {
    /// NOP on read
    fn read(_input: &BitSlice<u8, Msb0>, _inner_ctx: Ctx) -> Result<(usize, Self), DekuError>
    where
        Self: Sized,
    {
        Ok((0, ()))
    }
}

impl<Ctx: Copy> DekuReader<'_, Ctx> for () {
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        _inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        Ok(())
    }
}

impl<Ctx: Copy> DekuWrite<Ctx> for () {
    /// NOP on write
    fn write(&self, _output: &mut BitVec<u8, Msb0>, _inner_ctx: Ctx) -> Result<(), DekuError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hexlit::hex;

    use super::*;

    #[test]
    #[allow(clippy::unit_arg)]
    #[allow(clippy::unit_cmp)]
    fn test_unit() {
        let input = &hex!("FF");

        let bit_slice = input.view_bits::<Msb0>();
        let (amt_read, res_read) = <()>::read(bit_slice, ()).unwrap();
        assert_eq!((), res_read);
        assert_eq!(amt_read, 0);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(0, res_write.len());
    }
}
