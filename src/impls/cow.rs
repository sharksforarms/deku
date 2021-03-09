use crate::{DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;
use std::borrow::{Borrow, Cow};

impl<'a, T, Ctx> DekuRead<'a, Ctx> for Cow<'a, T>
where
    T: DekuRead<'a, Ctx> + Clone,
    Ctx: Copy,
{
    /// Read a T from input and store as Cow<T>
    fn read(
        input: &'a BitSlice<Msb0, u8>,
        inner_ctx: Ctx,
    ) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError>
    where
        Self: Sized,
    {
        let (rest, val) = <T>::read(input, inner_ctx)?;
        Ok((rest, Cow::Owned(val)))
    }
}

impl<T, Ctx> DekuWrite<Ctx> for Cow<'_, T>
where
    T: DekuWrite<Ctx> + Clone,
    Ctx: Copy,
{
    /// Write T from Cow<T>
    fn write(&self, output: &mut BitVec<Msb0, u8>, inner_ctx: Ctx) -> Result<(), DekuError> {
        (self.borrow() as &T).write(output, inner_ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest(input, expected, expected_rest,
        case(
            &[0xEF, 0xBE],
            Cow::Owned(0xBEEF),
            bits![Msb0, u8;]
        ),
    )]
    fn test_cow(input: &[u8], expected: Cow<u16>, expected_rest: &BitSlice<Msb0, u8>) {
        let bit_slice = input.view_bits::<Msb0>();
        let (rest, res_read) = <Cow<u16>>::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }
}
