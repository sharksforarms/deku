//! Implementations of DekuRead and DekuWrite for [T; N] where 0 < N <= 32

use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};
use core::mem::MaybeUninit;
use no_std_io::io::{Read, Seek, Write};

impl<'a, Ctx: Copy, T, const N: usize> DekuReader<'a, Ctx> for [T; N]
where
    T: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
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
            let value = match T::from_reader_with_ctx(reader, ctx) {
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
            // TODO: array_assume_init: https://github.com/rust-lang/rust/issues/96097
            (core::ptr::addr_of!(slice) as *const [T; N]).read()
        };
        Ok(val)
    }
}

impl<Ctx: Copy, T, const N: usize> DekuWriter<Ctx> for [T; N]
where
    T: DekuWriter<Ctx>,
{
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        for v in self {
            v.to_writer(writer, ctx)?;
        }
        Ok(())
    }
}

impl<Ctx: Copy, T> DekuWriter<Ctx> for &[T]
where
    T: DekuWriter<Ctx>,
{
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        for v in *self {
            v.to_writer(writer, ctx)?;
        }
        Ok(())
    }
}

impl<Ctx: Copy, T> DekuWriter<Ctx> for [T]
where
    T: DekuWriter<Ctx>,
{
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Ctx,
    ) -> Result<(), DekuError> {
        for v in self {
            v.to_writer(writer, ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::io::Cursor;

    use crate::{ctx::Endian, reader::Reader, writer::Writer, DekuReader};

    #[cfg(feature = "bits")]
    #[rstest(input,endian,expected,
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, [0xCCDD, 0xAABB]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, [0xDDCC, 0xBBAA]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 16 })")]
        case::normal_be([0xDD, 0xCC].as_ref(), Endian::Big, [0xDDCC, 0xBBAA]),
    )]
    fn test_bit_read(input: &[u8], endian: Endian, expected: [u16; 2]) {
        let mut cursor = std::io::Cursor::new(input);
        let mut reader = Reader::new(&mut cursor);
        let res_read = <[u16; 2]>::from_reader_with_ctx(&mut reader, endian).unwrap();
        assert_eq!(expected, res_read);
    }

    #[rstest(input,endian,expected,
        case::normal_le([0xDDCC, 0xBBAA], Endian::Little, vec![0xCC, 0xDD, 0xAA, 0xBB]),
        case::normal_be([0xDDCC, 0xBBAA], Endian::Big, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_write(input: [u16; 2], endian: Endian, expected: Vec<u8>) {
        // test writer

        use std::io::Cursor;
        let mut writer = Writer::new(Cursor::new(vec![]));
        input.to_writer(&mut writer, endian).unwrap();
        assert_eq!(expected, writer.inner.into_inner());
    }

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
        // test writer

        let mut writer = Writer::new(Cursor::new(vec![]));
        input.to_writer(&mut writer, endian).unwrap();
        assert_eq!(expected, writer.inner.into_inner());

        // test &slice
        let input = input.as_ref();
        let mut writer = Writer::new(Cursor::new(vec![]));
        input.to_writer(&mut writer, endian).unwrap();
        assert_eq!(expected, writer.inner.into_inner());
    }
}
