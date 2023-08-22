//! Implementations of DekuRead and DekuWrite for tuples of length 1 to 11

use acid_io::Read;
use bitvec::prelude::*;

use crate::{DekuError, DekuRead, DekuReader, DekuWrite};

// Trait to help us build intermediate tuples while DekuRead'ing each element
// from the tuple
trait Append<T> {
    type NewType;

    fn append(self, new: T) -> Self::NewType;
}

// Allow us to append an element of type T to a unit for the purpose
// of building a tuple
// Creates a tuple of length 1
impl<T> Append<T> for () {
    type NewType = (T,);

    fn append(self, new: T) -> Self::NewType {
        (new,)
    }
}

macro_rules! ImplDekuTupleTraits {
    ( $($T:ident,)+ ) => {
        impl<$($T,)+ U> Append<U> for ($($T,)+) {
            type NewType = ($($T,)+ U,);

            #[allow(non_snake_case)]
            fn append(self, new: U) -> Self::NewType {
                let ($($T,)+) = self;
                ($($T,)+ new,)
            }
        }

        impl<'a, Ctx: Copy, $($T:DekuRead<'a, Ctx>+Sized),+> DekuRead<'a, Ctx> for ($($T,)+)
        {
            fn read(
                input: &'a BitSlice<u8, Msb0>,
                ctx: Ctx,
            ) -> Result<(usize, Self), DekuError>
            where
                Self: Sized,
            {
                let tuple = ();
                let mut _rest = input;
                let mut total_read = 0;
                $(
                    let (amt_read, val) = <$T>::read(_rest, ctx)?;
                    let tuple = tuple.append(val);

                    total_read += amt_read;
                    _rest = &_rest[amt_read..];
                )+
                Ok((total_read, tuple))
            }
        }

        impl<'a, Ctx: Copy, $($T:DekuReader<'a, Ctx>+Sized),+> DekuReader<'a, Ctx> for ($($T,)+)
        {
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                ctx: Ctx,
            ) -> Result<Self, DekuError>
            where
                Self: Sized,
            {
                let tuple = ();
                $(
                    let val = <$T>::from_reader(container, ctx)?;
                    let tuple = tuple.append(val);
                )+
                Ok(tuple)
            }
        }

        impl<Ctx: Copy, $($T:DekuWrite<Ctx>),+> DekuWrite<Ctx> for ($($T,)+)
        {
            #[allow(non_snake_case)]
            fn write(&self, output: &mut BitVec<u8, Msb0>, ctx: Ctx) -> Result<(), DekuError> {
                let ($(ref $T,)+) = *self;
                $(
                    $T.write(output, ctx)?;
                )+
                Ok(())
            }
        }
    };
}

ImplDekuTupleTraits! { A, }
ImplDekuTupleTraits! { A, B, }
ImplDekuTupleTraits! { A, B, C, }
ImplDekuTupleTraits! { A, B, C, D, }
ImplDekuTupleTraits! { A, B, C, D, E, }
ImplDekuTupleTraits! { A, B, C, D, E, F, }
ImplDekuTupleTraits! { A, B, C, D, E, F, G, }
ImplDekuTupleTraits! { A, B, C, D, E, F, G, H, }
ImplDekuTupleTraits! { A, B, C, D, E, F, G, H, I, }
ImplDekuTupleTraits! { A, B, C, D, E, F, G, H, I, J, }
ImplDekuTupleTraits! { A, B, C, D, E, F, G, H, I, J, K, }

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;

    use super::*;
    use crate::native_endian;

    #[rstest(input, expected, expected_rest,
        case::length_1([0xef, 0xbe, 0xad, 0xde].as_ref(), (native_endian!(0xdeadbeef_u32),), bits![u8, Msb0;]),
        case::length_2([1, 0x24, 0x98, 0x82, 0].as_ref(), (true, native_endian!(0x829824_u32)), bits![u8, Msb0;]),
        case::length_11([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].as_ref(), (0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8), bits![u8, Msb0;]),
        case::extra_rest([1, 0x24, 0x98, 0x82, 0, 0].as_ref(), (true, native_endian!(0x829824_u32)), bits![u8, Msb0; 0, 0, 0, 0, 0, 0, 0, 0]),
    )]
    fn test_tuple_read<'a, T>(input: &'a [u8], expected: T, expected_rest: &BitSlice<u8, Msb0>)
    where
        T: DekuRead<'a> + Sized + PartialEq + Debug,
    {
        let bit_slice = input.view_bits::<Msb0>();
        let (amt_read, res_read) = <T>::read(bit_slice, ()).unwrap();
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);
    }

    #[rstest(input, expected,
        case::length_1((native_endian!(0xdeadbeef_u32),), vec![0xef, 0xbe, 0xad, 0xde]),
        case::length_2((true, native_endian!(0x829824_u32)), vec![1, 0x24, 0x98, 0x82, 0]),
        case::length_11((0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    )]
    fn test_tuple_write<T>(input: T, expected: Vec<u8>)
    where
        T: DekuWrite,
    {
        let mut res_write = bitvec![u8, Msb0;];
        input.write(&mut res_write, ()).unwrap();
        assert_eq!(expected, res_write.into_vec());
    }
}
