//! Implementations of DekuRead and DekuWrite for slices and arrays.

use crate::ctx::{BitSize, ByteSize, Limit, ReadExact};
use crate::reader::Reader;
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};
use core::mem::MaybeUninit;
use no_std_io::io::{Read, Seek, Write};

impl<'a> DekuReader<'a, ReadExact> for &'a [u8] {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        exact: ReadExact,
    ) -> Result<Self, DekuError> {
        reader.read_bytes_ref(exact.0)
    }
}

impl<'a, Ctx, Predicate: FnMut(&u8) -> bool> DekuReader<'a, (Limit<u8, Predicate>, Ctx)>
    for &'a [u8]
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        (limit, _inner_ctx): (Limit<u8, Predicate>, Ctx),
    ) -> Result<Self, DekuError> {
        match limit {
            Limit::Count(count) => reader.read_bytes_ref(count),
            Limit::ByteSize(ByteSize(size)) => reader.read_bytes_ref(size),
            Limit::BitSize(BitSize(size)) => {
                if !size.is_multiple_of(8) {
                    return Err(crate::deku_error!(
                        DekuError::InvalidParam,
                        "borrowed byte slices require a byte-aligned bit size"
                    ));
                }
                reader.read_bytes_ref(size / 8)
            }
            Limit::End => {
                let Some(source) = reader.source_bytes() else {
                    return Err(crate::deku_error!(
                        DekuError::InvalidParam,
                        "reading a borrowed slice to the end requires Reader::from_bytes"
                    ));
                };
                let start = reader.bits_read / 8;
                let size = source.len().saturating_sub(start);
                reader.read_bytes_ref(size)
            }
            Limit::Until(mut predicate, _) => {
                let start = reader.bits_read / 8;
                loop {
                    let byte = reader.read_bytes_ref(1)?[0];
                    if predicate(&byte) {
                        break;
                    }
                }
                let end = reader.bits_read / 8;
                let Some(source) = reader.source_bytes() else {
                    return Err(crate::deku_error!(
                        DekuError::InvalidParam,
                        "reading a borrowed slice until a predicate requires Reader::from_bytes"
                    ));
                };
                Ok(&source[start..end])
            }
        }
    }
}

impl<'a, Predicate: FnMut(&u8) -> bool> DekuReader<'a, Limit<u8, Predicate>> for &'a [u8] {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        limit: Limit<u8, Predicate>,
    ) -> Result<Self, DekuError> {
        <Self as DekuReader<'a, (Limit<u8, Predicate>, ())>>::from_reader_with_ctx(
            reader,
            (limit, ()),
        )
    }
}

impl<'a> DekuReader<'a, ByteSize> for &'a [u8] {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        size: ByteSize,
    ) -> Result<Self, DekuError> {
        reader.read_bytes_ref(size.0)
    }
}

impl<'a> DekuReader<'a, BitSize> for &'a [u8] {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        size: BitSize,
    ) -> Result<Self, DekuError> {
        if !size.0.is_multiple_of(8) {
            return Err(crate::deku_error!(
                DekuError::InvalidParam,
                "borrowed byte slices require a byte-aligned bit size"
            ));
        }
        reader.read_bytes_ref(size.0 / 8)
    }
}

impl<'a, Ctx: Copy, T, const N: usize> DekuReader<'a, Ctx> for [T; N]
where
    T: DekuReader<'a, Ctx>,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<'a, R>,
        ctx: Ctx,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let mut array: [MaybeUninit<T>; N] = [const { MaybeUninit::uninit() }; N];
        for (n, item) in array.iter_mut().enumerate() {
            match T::from_reader_with_ctx(reader, ctx) {
                Ok(value) => {
                    item.write(value);
                }
                Err(err) => {
                    // Drop initialized items
                    for item in &mut array[0..n] {
                        // SAFETY: `item` is certain to be initialized
                        unsafe {
                            item.assume_init_drop();
                        }
                    }

                    return Err(err);
                }
            };
        }

        // SAFETY: `array` is certain to be initialized
        let array = unsafe {
            // TODO: array_assume_init: https://github.com/rust-lang/rust/issues/96097
            (&raw const array).cast::<[T; N]>().read()
        };
        Ok(array)
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

impl<T: crate::DekuSize, const N: usize> crate::DekuSize for [T; N] {
    const SIZE_BITS: usize = T::SIZE_BITS * N;
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::io::Cursor;

    #[cfg(feature = "bits")]
    use crate::{DekuReader, reader::Reader};
    use crate::{ctx::Endian, writer::Writer};

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
