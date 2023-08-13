use acid_io::Read;
use bitvec::prelude::*;

use crate::{DekuError, DekuRead, DekuReader, DekuWrite};

impl<'a, T: DekuRead<'a, Ctx>, Ctx: Copy> DekuRead<'a, Ctx> for Option<T> {
    /// Read a T from input and store as Some(T)
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use deku::bitvec::BitView;
    /// let input = vec![1u8, 2, 3, 4];
    /// let (amt_read, v) = Option::<u32>::read(input.view_bits(), Endian::Little).unwrap();
    /// assert_eq!(amt_read, 32);
    /// assert_eq!(v, Some(0x04030201))
    /// ```
    fn read(input: &'a BitSlice<u8, Msb0>, inner_ctx: Ctx) -> Result<(usize, Self), DekuError>
    where
        Self: Sized,
    {
        let (amt_read, val) = <T>::read(input, inner_ctx)?;
        Ok((amt_read, Some(val)))
    }
}

impl<'a, T: DekuReader<'a, Ctx>, Ctx: Copy> DekuReader<'a, Ctx> for Option<T> {
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader(container, inner_ctx)?;
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
    #[test]
    fn test_option() {
        use crate::bitvec::BitView;
        use crate::ctx::*;
        use crate::DekuRead;
        let input = [1u8, 2, 3, 4];
        let (amt_read, v) = Option::<u32>::read(input.view_bits(), Endian::Little).unwrap();
        assert_eq!(amt_read, 32);
        assert_eq!(v, Some(0x04030201))
    }
}
