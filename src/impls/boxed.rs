use alloc::boxed::Box;
use alloc::vec::Vec;

use bitvec::prelude::*;

use crate::ctx::Limit;
use crate::{DekuError, DekuRead, DekuWrite};

impl<'a, T, Ctx> DekuRead<'a, Ctx> for Box<T>
where
    T: DekuRead<'a, Ctx>,
    Ctx: Copy,
{
    /// Read a T from input and store as Box<T>
    fn read(input: &'a BitSlice<u8, Msb0>, inner_ctx: Ctx) -> Result<(usize, Self), DekuError>
    where
        Self: Sized,
    {
        let (amt_read, val) = <T>::read(input, inner_ctx)?;
        Ok((amt_read, Box::new(val)))
    }
}

impl<T, Ctx> DekuWrite<Ctx> for Box<T>
where
    T: DekuWrite<Ctx>,
    Ctx: Copy,
{
    /// Write T from box
    fn write(&self, output: &mut BitVec<u8, Msb0>, inner_ctx: Ctx) -> Result<(), DekuError> {
        self.as_ref().write(output, inner_ctx)
    }
}

impl<'a, T, Ctx, Predicate> DekuRead<'a, (Limit<T, Predicate>, Ctx)> for Box<[T]>
where
    T: DekuRead<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(&T) -> bool,
{
    /// Read `T`s until the given limit
    fn read(
        input: &'a BitSlice<u8, Msb0>,
        (limit, inner_ctx): (Limit<T, Predicate>, Ctx),
    ) -> Result<(usize, Self), DekuError>
    where
        Self: Sized,
    {
        // use Vec<T>'s implementation and convert to Box<[T]>
        let (amt_read, val) = <Vec<T>>::read(input, (limit, inner_ctx))?;
        Ok((amt_read, val.into_boxed_slice()))
    }
}

impl<T, Ctx> DekuWrite<Ctx> for Box<[T]>
where
    T: DekuWrite<Ctx>,
    Ctx: Copy,
{
    /// Write all `T`s to bits
    fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
        for v in self.as_ref() {
            v.write(output, ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::ctx::*;
    use crate::native_endian;

    #[rstest(input, expected, expected_rest,
        case(
            &[0xEF, 0xBE],
            Box::new(native_endian!(0xBEEF_u16)),
            bits![u8, Msb0;]
        ),
    )]
    fn test_boxed(input: &[u8], expected: Box<u16>, expected_rest: &BitSlice<u8, Msb0>) {
        let bit_slice = input.view_bits::<Msb0>();
        let (amt_read, res_read) = <Box<u16>>::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    // Note: Copied tests from vec.rs impl
    #[rstest(input, endian, bit_size, limit, expected, expected_rest, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC].into_boxed_slice(), bits![u8, Msb0;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD].into_boxed_slice(), bits![u8, Msb0;], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA].into_boxed_slice(), bits![u8, Msb0; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB].into_boxed_slice(), bits![u8, Msb0; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), vec![0xBBAA].into_boxed_slice(), bits![u8, Msb0; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), vec![0xAABB].into_boxed_slice(), bits![u8, Msb0; 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1], vec![0xAA, 0xBB]),
    )]
    fn test_boxed_slice<Predicate: FnMut(&u16) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: Box<[u16]>,
        expected_rest: &BitSlice<u8, Msb0>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let (amt_read, res_read) =
            <Box<[u16]>>::read(bit_slice, (limit, (endian, BitSize(bit_size)))).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);

        let mut res_write = bitvec![u8, Msb0;];
        res_read
            .write(&mut res_write, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, res_write.into_vec());

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }
}
