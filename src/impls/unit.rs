use acid_io::Read;
use bitvec::prelude::*;

use crate::{DekuError, DekuReader, DekuWrite};

impl<Ctx: Copy> DekuReader<'_, Ctx> for () {
    fn from_reader_with_ctx<R: Read>(
        _reader: &mut crate::reader::Reader<R>,
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
    use crate::reader::Reader;
    use std::io::Cursor;

    use super::*;

    #[test]
    #[allow(clippy::unit_arg)]
    #[allow(clippy::unit_cmp)]
    fn test_unit() {
        let mut input = &[0xff];

        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = <()>::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!((), res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(0, res_write.len());
    }
}
