use bitvec::prelude::*;
use no_std_io::io::Read;

use crate::{DekuError, DekuReader, DekuWrite};

impl<'a, T: DekuReader<'a, Ctx>, Ctx: Copy> DekuReader<'a, Ctx> for Option<T> {
    fn from_reader_with_ctx<R: Read>(
        reader: &mut crate::reader::Reader<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader_with_ctx(reader, inner_ctx)?;
        Ok(Some(val))
    }
}

impl<T: DekuWrite<Ctx>, Ctx: Copy> DekuWrite<Ctx> for Option<T> {
    /// Write T if Some
    /// * **inner_ctx** - The context required by `T`.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite};
    /// # use deku::bitvec::{bitvec, Msb0};
    /// let data = Some(1u8);
    /// let mut output = bitvec![u8, Msb0;];
    /// data.write(&mut output, Endian::Big).unwrap();
    /// assert_eq!(output, bitvec![u8, Msb0; 0, 0, 0, 0, 0, 0, 0, 1])
    /// ```
    fn write(&self, output: &mut BitVec<u8, Msb0>, inner_ctx: Ctx) -> Result<(), DekuError> {
        self.as_ref().map_or(Ok(()), |v| v.write(output, inner_ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use no_std_io::io::Cursor;

    use crate::reader::Reader;

    #[test]
    fn test_option() {
        use crate::ctx::*;
        let input = &[1u8, 2, 3, 4];
        let mut cursor = Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let v = Option::<u32>::from_reader_with_ctx(&mut reader, Endian::Little).unwrap();
        assert_eq!(v, Some(0x04030201))
    }
}
