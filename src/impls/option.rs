use crate::{DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;

impl<'a, T: DekuRead<'a, Ctx>, Ctx: Copy> DekuRead<'a, Ctx> for Option<T> {
    /// Read a T from input and store as Some(T)
    /// * `inner_ctx` - The context required by `T`. It will be passed to every `T`s when constructing.
    /// # Examples
    /// ```rust
    /// # use deku::ctx::*;
    /// # use deku::DekuRead;
    /// # use bitvec::view::BitView;
    /// let input = vec![1u8, 2, 3, 4];
    /// let (rest, v) = Option::<u32>::read(input.view_bits(), Endian::Little).unwrap();
    /// assert!(rest.is_empty());
    /// assert_eq!(v, Some(0x04030201))
    /// ```
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        inner_ctx: Ctx,
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, val) = <T>::read(input, inner_ctx)?;
        Ok((rest, Some(val)))
    }
}

impl<T: DekuWrite<Ctx>, Ctx: Copy> DekuWrite<Ctx> for Option<T> {
    /// Write T if Some
    /// * **inner_ctx** - The context required by `T`.
    /// # Examples
    /// ```rust
    /// # use deku::{ctx::Endian, DekuWrite, prelude::{Lsb0, Msb0}};
    /// # use bitvec::bitvec;
    /// let data = Some(1u8);
    /// let mut output = bitvec![Msb0, u8;];
    /// data.write(&mut output, Endian::Big).unwrap();
    /// assert_eq!(output, bitvec![0, 0, 0, 0, 0, 0, 0, 1])
    /// ```
    fn write(&self, output: &mut BitVec<Msb0, u8>, inner_ctx: Ctx) -> Result<(), DekuError> {
        self.as_ref().map_or(Ok(()), |v| v.write(output, inner_ctx))
    }
}
