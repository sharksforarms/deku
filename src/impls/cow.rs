use std::borrow::{Borrow, Cow};

use acid_io::Read;

use bitvec::prelude::*;

use crate::{DekuError, DekuReader, DekuWrite};

impl<'a, T, Ctx> DekuReader<'a, Ctx> for Cow<'a, T>
where
    T: DekuReader<'a, Ctx> + Clone,
    Ctx: Copy,
{
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader(container, inner_ctx)?;
        Ok(Cow::Owned(val))
    }
}

impl<T, Ctx> DekuWrite<Ctx> for Cow<'_, T>
where
    T: DekuWrite<Ctx> + Clone,
    Ctx: Copy,
{
    /// Write T from Cow<T>
    fn write(&self, output: &mut BitVec<u8, Msb0>, inner_ctx: Ctx) -> Result<(), DekuError> {
        (self.borrow() as &T).write(output, inner_ctx)
    }
}

#[cfg(test)]
mod tests {
    use acid_io::Cursor;
    use rstest::rstest;

    use super::*;
    use crate::{container::Container, native_endian};

    #[rstest(input, expected, expected_rest,
        case(
            &[0xEF, 0xBE],
            Cow::Owned(native_endian!(0xBEEF_u16)),
            bits![u8, Msb0;]
        ),
    )]
    fn test_cow(input: &[u8], expected: Cow<u16>, expected_rest: &BitSlice<u8, Msb0>) {
        let bit_slice = input.view_bits::<Msb0>();

        let mut cursor = Cursor::new(input);
        let mut container = Container::new(&mut cursor);
        let res_read = <Cow<u16>>::from_reader(&mut container, ()).unwrap();
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }
}
