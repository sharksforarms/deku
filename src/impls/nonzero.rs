use acid_io::Read;
#[cfg(feature = "alloc")]
use alloc::format;
use core::num::*;

use bitvec::prelude::*;

use crate::ctx::*;
use crate::{DekuError, DekuReader, DekuWrite};

macro_rules! ImplDekuTraitsCtx {
    ($typ:ty, $readtype:ty, $ctx_arg:tt, $ctx_type:tt) => {
        impl DekuReader<'_, $ctx_type> for $typ {
            fn from_reader_with_ctx<R: Read>(
                reader: &mut crate::reader::Reader<R>,
                $ctx_arg: $ctx_type,
            ) -> Result<Self, DekuError> {
                let value = <$readtype>::from_reader_with_ctx(reader, $ctx_arg)?;
                let value = <$typ>::new(value);

                match value {
                    None => Err(DekuError::Parse(format!("NonZero assertion"))),
                    Some(v) => Ok(v),
                }
            }
        }

        impl DekuWrite<$ctx_type> for $typ {
            fn write(
                &self,
                output: &mut BitVec<u8, Msb0>,
                $ctx_arg: $ctx_type,
            ) -> Result<(), DekuError> {
                let value = self.get();
                value.write(output, $ctx_arg)
            }
        }
    };
}

macro_rules! ImplDekuTraits {
    ($typ:ty, $readtype:ty) => {
        ImplDekuTraitsCtx!($typ, $readtype, (), ());
        ImplDekuTraitsCtx!($typ, $readtype, (endian, bitsize), (Endian, BitSize));
        ImplDekuTraitsCtx!($typ, $readtype, (endian, bytesize), (Endian, ByteSize));
        ImplDekuTraitsCtx!($typ, $readtype, endian, Endian);
    };
}

ImplDekuTraits!(NonZeroU8, u8);
ImplDekuTraits!(NonZeroU16, u16);
ImplDekuTraits!(NonZeroU32, u32);
ImplDekuTraits!(NonZeroU64, u64);
ImplDekuTraits!(NonZeroU128, u128);
ImplDekuTraits!(NonZeroUsize, usize);
ImplDekuTraits!(NonZeroI8, i8);
ImplDekuTraits!(NonZeroI16, i16);
ImplDekuTraits!(NonZeroI32, i32);
ImplDekuTraits!(NonZeroI64, i64);
ImplDekuTraits!(NonZeroI128, i128);
ImplDekuTraits!(NonZeroIsize, isize);

#[cfg(test)]
mod tests {
    use hexlit::hex;
    use rstest::rstest;

    use crate::reader::Reader;

    use super::*;

    #[rstest(input, expected,
        case(&hex!("FF"), NonZeroU8::new(0xFF).unwrap()),

        #[should_panic(expected = "Parse(\"NonZero assertion\")")]
        case(&hex!("00"), NonZeroU8::new(0xFF).unwrap()),
    )]
    fn test_non_zero(input: &[u8], expected: NonZeroU8) {
        let mut bit_slice = input.view_bits::<Msb0>();

        let mut reader = Reader::new(&mut bit_slice);
        let res_read = NonZeroU8::from_reader_with_ctx(&mut reader, ()).unwrap();
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        res_read.write(&mut res_write, ()).unwrap();
        assert_eq!(input.to_vec(), res_write.into_vec());
    }
}
