//! Implementations of DekuRead and DekuWrite for [T; N] where 0 < N <= 32

pub use deku_derive::*;

#[cfg(feature = "const_generics")]
mod const_generics_impl {
    use crate::{DekuError, DekuWrite};
    use bitvec::prelude::*;
    use core::mem::MaybeUninit;
    use std::io::Read;

    use crate::DekuReader;

    impl<'a, Ctx: Copy, T, const N: usize> DekuReader<'a, Ctx> for [T; N]
    where
        T: DekuReader<'a, Ctx>,
    {
        fn from_reader<R: Read>(
            container: &mut crate::container::Container<R>,
            ctx: Ctx,
        ) -> Result<Self, DekuError>
        where
            Self: Sized,
        {
            #[allow(clippy::uninit_assumed_init)]
            // This is safe because we initialize the array immediately after,
            // and never return it in case of error
            let mut slice: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
            for (n, item) in slice.iter_mut().enumerate() {
                let value = match T::from_reader(container, ctx) {
                    Ok(it) => it,
                    Err(err) => {
                        // For each item in the array, drop if we allocated it.
                        for item in &mut slice[0..n] {
                            unsafe {
                                item.assume_init_drop();
                            }
                        }
                        return Err(err);
                    }
                };
                item.write(value);
            }

            let val = unsafe {
                // TODO: array_assume_init: https://github.com/rust-lang/rust/issues/80908
                (std::ptr::addr_of!(slice) as *const [T; N]).read()
            };
            Ok(val)
        }
    }

    impl<Ctx: Copy, T, const N: usize> DekuWrite<Ctx> for [T; N]
    where
        T: DekuWrite<Ctx>,
    {
        fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
            for v in self {
                v.write(output, ctx)?;
            }
            Ok(())
        }
    }

    impl<Ctx: Copy, T> DekuWrite<Ctx> for &[T]
    where
        T: DekuWrite<Ctx>,
    {
        fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
            for v in *self {
                v.write(output, ctx)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::DekuWrite;
    use bitvec::prelude::*;
    use rstest::rstest;

    use crate::{container::Container, ctx::Endian, DekuReader};

    #[rstest(input,endian,expected,expected_rest,
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, [0xCCDD, 0xAABB], bits![u8, Msb0;]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, [0xDDCC, 0xBBAA], bits![u8, Msb0;]),
    )]
    fn test_bit_read(
        input: &[u8],
        endian: Endian,
        expected: [u16; 2],
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let mut bit_slice = input.view_bits::<Msb0>();

        let mut container = Container::new(&mut bit_slice);
        let res_read = <[u16; 2]>::from_reader(&mut container, endian).unwrap();
        assert_eq!(expected, res_read);
    }

    #[rstest(input,endian,expected,
        case::normal_le([0xDDCC, 0xBBAA], Endian::Little, vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xDDCC, 0xBBAA], Endian::Big, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_write(input: [u16; 2], endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());

        // test &slice
        let input = input.as_ref();
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }

    #[cfg(feature = "const_generics")]
    #[rstest(input,endian,expected,expected_rest,
        case::normal_le(
            [0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66].as_ref(),
            Endian::Little,
            [[0xCCDD, 0xAABB], [0x8899, 0x6677]],
            bits![u8, Msb0;],
        ),
        case::normal_le(
            [0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66].as_ref(),
            Endian::Big,
            [[0xDDCC, 0xBBAA], [0x9988, 0x7766]],
            bits![u8, Msb0;],
        ),
    )]
    fn test_nested_array_bit_read(
        input: &[u8],
        endian: Endian,
        expected: [[u16; 2]; 2],
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        use acid_io::Cursor;

        use crate::container::Container;

        let bit_slice = input.view_bits::<Msb0>();

        let mut cursor = Cursor::new(input);
        let mut container = Container::new(&mut cursor);
        let res_read = <[[u16; 2]; 2]>::from_reader(&mut container, endian).unwrap();
        assert_eq!(expected, res_read);
    }

    #[cfg(feature = "const_generics")]
    #[rstest(input,endian,expected,
        case::normal_le(
            [[0xCCDD, 0xAABB], [0x8899, 0x6677]],
            Endian::Little,
            vec![0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66],
        ),
        case::normal_be(
            [[0xDDCC, 0xBBAA], [0x9988, 0x7766]],
            Endian::Big,
            vec![0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66],
        ),
    )]
    fn test_nested_array_bit_write(input: [[u16; 2]; 2], endian: Endian, expected: Vec<u8>) {
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());

        // test &slice
        let input = input.as_ref();
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, endian).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }
}
