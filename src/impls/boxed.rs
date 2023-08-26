use acid_io::Read;

use alloc::boxed::Box;
use alloc::vec::Vec;

use bitvec::prelude::*;

use crate::ctx::Limit;
use crate::{DekuError, DekuReader, DekuWrite};

impl<'a, T, Ctx> DekuReader<'a, Ctx> for Box<T>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
{
    fn from_reader_with_ctx<R: Read>(
        container: &mut crate::container::Container<R>,
        inner_ctx: Ctx,
    ) -> Result<Self, DekuError> {
        let val = <T>::from_reader_with_ctx(container, inner_ctx)?;
        Ok(Box::new(val))
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

impl<'a, T, Ctx, Predicate> DekuReader<'a, (Limit<T, Predicate>, Ctx)> for Box<[T]>
where
    T: DekuReader<'a, Ctx>,
    Ctx: Copy,
    Predicate: FnMut(&T) -> bool,
{
    fn from_reader_with_ctx<R: Read>(
        container: &mut crate::container::Container<R>,
        (limit, inner_ctx): (Limit<T, Predicate>, Ctx),
    ) -> Result<Self, DekuError> {
        // use Vec<T>'s implementation and convert to Box<[T]>
        let val = <Vec<T>>::from_reader_with_ctx(container, (limit, inner_ctx))?;
        Ok(val.into_boxed_slice())
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
    use acid_io::Cursor;
    use rstest::rstest;

    use super::*;
    use crate::container::Container;
    use crate::ctx::*;
    use crate::native_endian;

    #[rstest(input, expected,
        case(
            &[0xEF, 0xBE],
            Box::new(native_endian!(0xBEEF_u16)),
        ),
    )]
    fn test_boxed(input: &[u8], expected: Box<u16>) {
        let mut cursor = Cursor::new(input);
        let mut container = Container::new(&mut cursor);
        let res_read = <Box<u16>>::from_reader_with_ctx(&mut container, ()).unwrap();
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }

    // Note: Copied tests from vec.rs impl
    #[rstest(input, endian, bit_size, limit, expected, expected_rest_bits, expected_rest_bytes, expected_write,
        case::normal_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), 2.into(), vec![0xBBAA, 0xDDCC].into_boxed_slice(), bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), 2.into(), vec![0xAABB, 0xCCDD].into_boxed_slice(), bits![u8, Msb0;], &[], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::predicate_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), (|v: &u16| *v == 0xBBAA).into(), vec![0xBBAA].into_boxed_slice(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::predicate_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), (|v: &u16| *v == 0xAABB).into(), vec![0xAABB].into_boxed_slice(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_le([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(16), BitSize(16).into(), vec![0xBBAA].into_boxed_slice(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
        case::bytes_be([0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Big, Some(16), BitSize(16).into(), vec![0xAABB].into_boxed_slice(), bits![u8, Msb0;], &[0xcc, 0xdd], vec![0xAA, 0xBB]),
    )]
    fn test_boxed_slice_from_reader_with_ctx<Predicate: FnMut(&u16) -> bool>(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        limit: Limit<u16, Predicate>,
        expected: Box<[u16]>,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
        expected_write: Vec<u8>,
    ) {
        // Unwrap here because all test cases are `Some`.
        let bit_size = bit_size.unwrap();

        let mut cursor = Cursor::new(input);
        let mut container = Container::new(&mut cursor);
        let res_read = <Box<[u16]>>::from_reader_with_ctx(
            &mut container,
            (limit, (endian, BitSize(bit_size))),
        )
        .unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(
            container.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        cursor.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);

        let mut res_write = bitvec![u8, Msb0;];
        res_read
            .write(&mut res_write, (endian, BitSize(bit_size)))
            .unwrap();
        assert_eq!(expected_write, res_write.into_vec());

        assert_eq!(input[..expected_write.len()].to_vec(), expected_write);
    }
}
