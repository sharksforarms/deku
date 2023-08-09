#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::string::ToString;
use core::convert::TryInto;

use acid_io::Read;
use bitvec::prelude::*;

use crate::container::ContainerRet;
use crate::ctx::*;
use crate::prelude::NeedSize;
use crate::{DekuError, DekuRead, DekuReader, DekuWrite};

// specialize u8 for ByteSize
impl DekuRead<'_, (Endian, ByteSize)> for u8 {
    #[inline]
    fn read(
        input: &BitSlice<u8, Msb0>,
        (_, _): (Endian, ByteSize),
    ) -> Result<(usize, Self), DekuError> {
        const MAX_TYPE_BITS: usize = BitSize::of::<u8>().0;
        if input.len() < MAX_TYPE_BITS {
            return Err(DekuError::Incomplete(NeedSize::new(MAX_TYPE_BITS)));
        }

        // PANIC: We already check that input.len() < bit_size above, so no panic will happen
        let value = input[..MAX_TYPE_BITS].load::<u8>();
        Ok((MAX_TYPE_BITS, value))
    }
}

impl DekuReader<'_, (Endian, ByteSize)> for u8 {
    // specialize not needed?
    #[inline]
    fn from_reader<R: Read>(
        container: &mut crate::container::Container<R>,
        (endian, size): (Endian, ByteSize),
    ) -> Result<u8, DekuError> {
        let mut buf = [0; core::mem::size_of::<u8>()];
        let ret = container.read_bytes(size.0, &mut buf)?;
        let a = match ret {
            ContainerRet::Bits(bits) => {
                let Some(bits) = bits else {
                    return Err(DekuError::Parse("no bits read from reader".to_string()));
                };
                let a = <u8>::read(&bits, (endian, size))?;
                a.1
            }
            ContainerRet::Bytes => <u8>::from_be_bytes(buf),
        };
        Ok(a)
    }
}

macro_rules! ImplDekuReadBits {
    ($typ:ty, $inner:ty) => {
        impl DekuRead<'_, (Endian, BitSize)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size): (Endian, BitSize),
            ) -> Result<(usize, Self), DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size: usize = size.0;

                let input_is_le = endian.is_le();

                if bit_size > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {bit_size} bits",
                    )));
                }

                if input.len() < bit_size {
                    return Err(DekuError::Incomplete(crate::error::NeedSize::new(bit_size)));
                }

                // PANIC: We already check that input.len() < bit_size above, so no panic will happen
                let bit_slice = &input[..bit_size];

                let pad = 8 * ((bit_slice.len() + 7) / 8) - bit_slice.len();

                // if everything is aligned, just read the value
                if pad == 0 && bit_slice.len() == MAX_TYPE_BITS {
                    let bytes = bit_slice.domain().region().unwrap().1;

                    if bytes.len() * 8 == MAX_TYPE_BITS {
                        // Read value
                        let value = if input_is_le {
                            <$typ>::from_le_bytes(bytes.try_into()?)
                        } else {
                            <$typ>::from_be_bytes(bytes.try_into()?)
                        };
                        return Ok((bit_size, value));
                    }
                }

                // Create a new BitVec from the slice and pad un-aligned chunks
                // i.e. [10010110, 1110] -> [10010110, 00001110]
                let bits: BitVec<u8, Msb0> = {
                    let mut bits = BitVec::with_capacity(bit_slice.len() + pad);

                    // Copy bits to new BitVec
                    bits.extend_from_bitslice(&bit_slice);

                    // Force align
                    //i.e. [1110, 10010110] -> [11101001, 0110]
                    bits.force_align();

                    // Some padding to next byte
                    let index = if input_is_le {
                        bits.len() - (8 - pad)
                    } else {
                        0
                    };
                    for _ in 0..pad {
                        bits.insert(index, false);
                    }

                    // Pad up-to size of type
                    for _ in 0..(MAX_TYPE_BITS - bits.len()) {
                        if input_is_le {
                            bits.push(false);
                        } else {
                            bits.insert(0, false);
                        }
                    }

                    bits
                };

                let bytes: &[u8] = bits.domain().region().unwrap().1;

                // Read value
                let value = if input_is_le {
                    <$typ>::from_le_bytes(bytes.try_into()?)
                } else {
                    <$typ>::from_be_bytes(bytes.try_into()?)
                };
                Ok((bit_size, value))
            }
        }

        impl DekuReader<'_, (Endian, BitSize)> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                (endian, size): (Endian, BitSize),
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                if size.0 > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {} bits", size.0
                    )));
                }
                let bits = container.read_bits(size.0)?;
                let Some(bits) = bits else {
                    return Err(DekuError::Parse(format!(
                        "no bits read from reader",
                    )));
                };
                let a = <$typ>::read(&bits, (endian, size))?;
                Ok(a.1)
            }
        }
    };
}

macro_rules! ImplDekuReadBytes {
    ($typ:ty, $inner:ty) => {
        impl DekuRead<'_, (Endian, ByteSize)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<(usize, Self), DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size: usize = size.0 * 8;

                let input_is_le = endian.is_le();

                if bit_size > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {bit_size} bits",
                    )));
                }

                if input.len() < bit_size {
                    return Err(DekuError::Incomplete(crate::error::NeedSize::new(bit_size)));
                }

                let bit_slice = &input[..bit_size];

                let pad = 8 * ((bit_slice.len() + 7) / 8) - bit_slice.len();

                let bytes = bit_slice.domain().region().unwrap().1;
                let value = if pad == 0
                    && bit_slice.len() == MAX_TYPE_BITS
                    && bytes.len() * 8 == MAX_TYPE_BITS
                {
                    // if everything is aligned, just read the value
                    if input_is_le {
                        <$typ>::from_le_bytes(bytes.try_into()?)
                    } else {
                        <$typ>::from_be_bytes(bytes.try_into()?)
                    }
                } else {
                    let mut bits: BitVec<u8, Msb0> = BitVec::with_capacity(bit_slice.len() + pad);

                    // Copy bits to new BitVec
                    bits.extend_from_bitslice(&bit_slice);

                    // Force align
                    //i.e. [1110, 10010110] -> [11101001, 0110]
                    bits.force_align();

                    // cannot use from_X_bytes as we don't have enough bytes for $typ
                    // read manually
                    let mut res: $inner = 0;
                    if input_is_le {
                        for b in bytes.iter().rev() {
                            res <<= 8 as $inner;
                            res |= *b as $inner;
                        }
                    } else {
                        for b in bytes.iter() {
                            res <<= 8 as $inner;
                            res |= *b as $inner;
                        }
                    };

                    res as $typ
                };

                Ok((bit_size, value))
            }
        }

        impl DekuReader<'_, (Endian, ByteSize)> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<$typ, DekuError> {
                let mut buf = [0; core::mem::size_of::<$typ>()];
                let ret = container.read_bytes(size.0, &mut buf)?;
                let a = match ret {
                    ContainerRet::Bits(bits) => {
                        let Some(bits) = bits else {
                            return Err(DekuError::Parse(format!(
                                "no bits read from reader",
                            )));
                        };
                        let a = <$typ>::read(&bits, (endian, size))?;
                        a.1
                    }
                    ContainerRet::Bytes => {
                        if endian.is_le() {
                            <$typ>::from_le_bytes(buf.try_into().unwrap())
                        } else {
                            if size.0 != core::mem::size_of::<$typ>() {
                                let padding = core::mem::size_of::<$typ>() - size.0;
                                buf.copy_within(0..size.0, padding);
                                buf[..padding].fill(0x00);
                            }
                            <$typ>::from_be_bytes(buf.try_into().unwrap())
                        }
                    }
                };
                Ok(a)
            }
        }
    };
}

macro_rules! ImplDekuReadSignExtend {
    ($typ:ty, $inner:ty) => {
        impl DekuRead<'_, (Endian, ByteSize)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<(usize, Self), DekuError> {
                let (amt_read, value) =
                    <$inner as DekuRead<'_, (Endian, ByteSize)>>::read(input, (endian, size))?;

                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size = size.0 * 8;
                let shift = MAX_TYPE_BITS - bit_size;
                let value = (value as $typ) << shift >> shift;
                Ok((amt_read, value))
            }
        }

        impl DekuReader<'_, (Endian, ByteSize)> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<$typ, DekuError> {
                // TODO: specialize for reading byte aligned
                let bits = container.read_bits(size.0 * 8)?;
                let Some(bits) = bits else {
                            return Err(DekuError::Parse(format!("no bits read from reader",)));
                        };
                let a = <$typ>::read(&bits, (endian, size))?;
                Ok(a.1)
            }
        }

        impl DekuRead<'_, (Endian, BitSize)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size): (Endian, BitSize),
            ) -> Result<(usize, Self), DekuError> {
                let (amt_read, value) =
                    <$inner as DekuRead<'_, (Endian, BitSize)>>::read(input, (endian, size))?;

                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size = size.0;
                let shift = MAX_TYPE_BITS - bit_size;
                let value = (value as $typ) << shift >> shift;
                Ok((amt_read, value))
            }
        }

        impl DekuReader<'_, (Endian, BitSize)> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                (endian, size): (Endian, BitSize),
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                if size.0 > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {} bits", size.0
                    )));
                }
                let bits = container.read_bits(size.0)?;
                let Some(bits) = bits else {
                            return Err(DekuError::Parse(format!("no bits read from reader",)));
                        };
                let a = <$typ>::read(&bits, (endian, size))?;
                Ok(a.1)
            }
        }
    };
}

macro_rules! ForwardDekuRead {
    ($typ:ty) => {
        // Only have `endian`, set `bit_size` to `Size::of::<Type>()`
        impl DekuRead<'_, Endian> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                endian: Endian,
            ) -> Result<(usize, Self), DekuError> {
                let bit_size = BitSize::of::<$typ>();

                // Since we don't have a #[bits] or [bytes], check if we can use bytes for perf
                if (bit_size.0 % 8) == 0 {
                    <$typ>::read(input, (endian, ByteSize(bit_size.0 / 8)))
                } else {
                    <$typ>::read(input, (endian, bit_size))
                }
            }
        }

        impl DekuReader<'_, Endian> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                endian: Endian,
            ) -> Result<$typ, DekuError> {
                let bit_size = BitSize::of::<$typ>();

                // Since we don't have a #[bits] or [bytes], check if we can use bytes for perf
                let a = if (bit_size.0 % 8) == 0 {
                    <$typ>::from_reader(container, (endian, ByteSize(bit_size.0 / 8)))?
                } else {
                    <$typ>::from_reader(container, (endian, bit_size))?
                };
                Ok(a)
            }
        }

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuRead<'_, ByteSize> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                byte_size: ByteSize,
            ) -> Result<(usize, Self), DekuError> {
                let endian = Endian::default();

                <$typ>::read(input, (endian, byte_size))
            }
        }

        impl DekuReader<'_, ByteSize> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                byte_size: ByteSize,
            ) -> Result<$typ, DekuError> {
                let endian = Endian::default();

                let a = <$typ>::from_reader(container, (endian, byte_size))?;
                Ok(a)
            }
        }

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuRead<'_, BitSize> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                bit_size: BitSize,
            ) -> Result<(usize, Self), DekuError> {
                let endian = Endian::default();

                // check if we can use ByteSize for performance
                if (bit_size.0 % 8) == 0 {
                    <$typ>::read(input, (endian, ByteSize(bit_size.0 / 8)))
                } else {
                    <$typ>::read(input, (endian, bit_size))
                }
            }
        }

        impl DekuReader<'_, BitSize> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                bit_size: BitSize,
            ) -> Result<$typ, DekuError> {
                let endian = Endian::default();

                let a = <$typ>::from_reader(container, (endian, bit_size))?;
                Ok(a)
            }
        }

        impl DekuRead<'_> for $typ {
            #[inline]
            fn read(input: &BitSlice<u8, Msb0>, _: ()) -> Result<(usize, Self), DekuError> {
                <$typ>::read(input, Endian::default())
            }
        }

        impl DekuReader<'_> for $typ {
            #[inline]
            fn from_reader<R: Read>(
                container: &mut crate::container::Container<R>,
                _: (),
            ) -> Result<$typ, DekuError> {
                <$typ>::from_reader(container, Endian::default())
            }
        }
    };
}

macro_rules! ImplDekuWrite {
    ($typ:ty) => {
        impl DekuWrite<(Endian, BitSize)> for $typ {
            #[inline]
            fn write(
                &self,
                output: &mut BitVec<u8, Msb0>,
                (endian, size): (Endian, BitSize),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                let bit_size: usize = size.0;

                let input_bits = input.view_bits::<Msb0>();

                if bit_size > input_bits.len() {
                    return Err(DekuError::InvalidParam(format!(
                        "bit size {} is larger then input {}",
                        bit_size,
                        input_bits.len()
                    )));
                }

                if matches!(endian, Endian::Little) {
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [10101011, 00000011, 00000000, 00000000]
                    let mut remaining_bits = bit_size;
                    for chunk in input_bits.chunks(8) {
                        if chunk.len() > remaining_bits {
                            output.extend_from_bitslice(&chunk[chunk.len() - remaining_bits..]);
                            break;
                        } else {
                            output.extend_from_bitslice(chunk)
                        }
                        remaining_bits -= chunk.len();
                    }
                } else {
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [00000000, 00000000, 00000010, 10101111]
                    output.extend_from_bitslice(&input_bits[input_bits.len() - bit_size..]);
                }
                Ok(())
            }
        }

        impl DekuWrite<(Endian, ByteSize)> for $typ {
            #[inline]
            fn write(
                &self,
                output: &mut BitVec<u8, Msb0>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                let bit_size: usize = size.0 * 8;

                let input_bits = input.view_bits::<Msb0>();

                if bit_size > input_bits.len() {
                    return Err(DekuError::InvalidParam(format!(
                        "bit size {} is larger then input {}",
                        bit_size,
                        input_bits.len()
                    )));
                }

                if matches!(endian, Endian::Little) {
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [10101011, 00000011, 00000000, 00000000]
                    let mut remaining_bits = bit_size;
                    for chunk in input_bits.chunks(8) {
                        if chunk.len() > remaining_bits {
                            output.extend_from_bitslice(&chunk[chunk.len() - remaining_bits..]);
                            break;
                        } else {
                            output.extend_from_bitslice(chunk)
                        }
                        remaining_bits -= chunk.len();
                    }
                } else {
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [00000000, 00000000, 00000010, 10101111]
                    output.extend_from_bitslice(&input_bits[input_bits.len() - bit_size..]);
                }
                Ok(())
            }
        }

        // Only have `endian`, return all input
        impl DekuWrite<Endian> for $typ {
            #[inline]
            fn write(
                &self,
                output: &mut BitVec<u8, Msb0>,
                endian: Endian,
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };
                output.extend_from_bitslice(input.view_bits::<Msb0>());
                Ok(())
            }
        }
    };
}

macro_rules! ForwardDekuWrite {
    ($typ:ty) => {
        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuWrite<BitSize> for $typ {
            #[inline]
            fn write(
                &self,
                output: &mut BitVec<u8, Msb0>,
                bit_size: BitSize,
            ) -> Result<(), DekuError> {
                <$typ>::write(self, output, (Endian::default(), bit_size))
            }
        }

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuWrite<ByteSize> for $typ {
            #[inline]
            fn write(
                &self,
                output: &mut BitVec<u8, Msb0>,
                bit_size: ByteSize,
            ) -> Result<(), DekuError> {
                <$typ>::write(self, output, (Endian::default(), bit_size))
            }
        }

        impl DekuWrite for $typ {
            #[inline]
            fn write(&self, output: &mut BitVec<u8, Msb0>, _: ()) -> Result<(), DekuError> {
                <$typ>::write(self, output, Endian::default())
            }
        }
    };
}
macro_rules! ImplDekuTraitsBytes {
    ($typ:ty) => {
        ImplDekuReadBytes!($typ, $typ);
    };
    ($typ:ty, $inner:ty) => {
        ImplDekuReadBytes!($typ, $inner);
    };
}

macro_rules! ImplDekuTraits {
    ($typ:ty) => {
        ImplDekuReadBits!($typ, $typ);
        ForwardDekuRead!($typ);

        ImplDekuWrite!($typ);
        ForwardDekuWrite!($typ);
    };
    ($typ:ty, $inner:ty) => {
        ImplDekuReadBits!($typ, $inner);
        ForwardDekuRead!($typ);

        ImplDekuWrite!($typ);
        ForwardDekuWrite!($typ);
    };
}

macro_rules! ImplDekuTraitsSignExtend {
    ($typ:ty, $inner:ty) => {
        ImplDekuReadSignExtend!($typ, $inner);
        ForwardDekuRead!($typ);

        ImplDekuWrite!($typ);
        ForwardDekuWrite!($typ);
    };
}

ImplDekuTraits!(u8);
ImplDekuTraits!(u16);
ImplDekuTraitsBytes!(u16);
ImplDekuTraits!(u32);
ImplDekuTraitsBytes!(u32);
ImplDekuTraits!(u64);
ImplDekuTraitsBytes!(u64);
ImplDekuTraits!(u128);
ImplDekuTraitsBytes!(u128);
ImplDekuTraits!(usize);
ImplDekuTraitsBytes!(usize);

ImplDekuTraitsSignExtend!(i8, u8);
ImplDekuTraitsSignExtend!(i16, u16);
ImplDekuTraitsSignExtend!(i32, u32);
ImplDekuTraitsSignExtend!(i64, u64);
ImplDekuTraitsSignExtend!(i128, u128);
ImplDekuTraitsSignExtend!(isize, usize);

ImplDekuTraits!(f32, u32);
ImplDekuTraitsBytes!(f32, u32);
ImplDekuTraits!(f64, u64);
ImplDekuTraitsBytes!(f64, u64);

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{container::Container, native_endian};

    static ENDIAN: Endian = Endian::new();

    macro_rules! TestPrimitive {
        ($test_name:ident, $typ:ty, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let input = $input;
                let bit_slice = input.view_bits::<Msb0>();
                let (_rest, res_read) = <$typ>::read(bit_slice, ENDIAN).unwrap();
                assert_eq!($expected, res_read);

                let res_read = <$typ>::from_reader(&mut Container::new(bit_slice), ENDIAN).unwrap();
                assert_eq!($expected, res_read);

                let mut res_write = bitvec![u8, Msb0;];
                res_read.write(&mut res_write, ENDIAN).unwrap();
                assert_eq!(input, res_write.into_vec());
            }
        };
    }

    TestPrimitive!(test_u8, u8, vec![0xaau8], 0xaau8);
    TestPrimitive!(
        test_u16,
        u16,
        vec![0xabu8, 0xcd],
        native_endian!(0xcdab_u16)
    );
    TestPrimitive!(
        test_u32,
        u32,
        vec![0xabu8, 0xcd, 0xef, 0xbe],
        native_endian!(0xbeefcdab_u32)
    );
    TestPrimitive!(
        test_u64,
        u64,
        vec![0xabu8, 0xcd, 0xef, 0xbe, 0xab, 0xcd, 0xfe, 0xc0],
        native_endian!(0xc0fecdabbeefcdab_u64)
    );
    TestPrimitive!(
        test_u128,
        u128,
        vec![
            0xabu8, 0xcd, 0xef, 0xbe, 0xab, 0xcd, 0xfe, 0xc0, 0xab, 0xcd, 0xef, 0xbe, 0xab, 0xcd,
            0xfe, 0xc0
        ],
        native_endian!(0xc0fecdabbeefcdabc0fecdabbeefcdab_u128)
    );
    TestPrimitive!(
        test_usize,
        usize,
        vec![0xabu8, 0xcd, 0xef, 0xbe, 0xab, 0xcd, 0xfe, 0xc0],
        if core::mem::size_of::<usize>() == 8 {
            native_endian!(0xc0fecdabbeefcdab_usize)
        } else {
            native_endian!(0xbeefcdab_usize)
        }
    );
    TestPrimitive!(test_i8, i8, vec![0xfbu8], -5);
    TestPrimitive!(test_i16, i16, vec![0xfdu8, 0xfe], native_endian!(-259_i16));
    TestPrimitive!(
        test_i32,
        i32,
        vec![0x02u8, 0x3f, 0x01, 0xef],
        native_endian!(-0x10fec0fe_i32)
    );
    TestPrimitive!(
        test_i64,
        i64,
        vec![0x02u8, 0x3f, 0x01, 0xef, 0x01, 0x3f, 0x01, 0xef],
        native_endian!(-0x10fec0fe10fec0fe_i64)
    );
    TestPrimitive!(
        test_i128,
        i128,
        vec![
            0x02u8, 0x3f, 0x01, 0xef, 0x01, 0x3f, 0x01, 0xef, 0x01, 0x3f, 0x01, 0xef, 0x01, 0x3f,
            0x01, 0xef
        ],
        native_endian!(-0x10fec0fe10fec0fe10fec0fe10fec0fe_i128)
    );
    TestPrimitive!(
        test_isize,
        isize,
        vec![0x02u8, 0x3f, 0x01, 0xef, 0x01, 0x3f, 0x01, 0xef],
        if core::mem::size_of::<isize>() == 8 {
            native_endian!(-0x10fec0fe10fec0fe_isize)
        } else {
            native_endian!(-0x10fec0fe_isize)
        }
    );
    TestPrimitive!(
        test_f32,
        f32,
        vec![0xa6u8, 0x9b, 0xc4, 0xbb],
        native_endian!(-0.006_f32)
    );
    TestPrimitive!(
        test_f64,
        f64,
        vec![0xfau8, 0x7e, 0x6a, 0xbc, 0x74, 0x93, 0x78, 0xbf],
        native_endian!(-0.006_f64)
    );

    #[rstest(bit_slice, endian, bit_size, expected, expected_rest,
        case::normal(bits![u8, Msb0; 1], Endian::Little, Some(1), 0x1, bits![u8, Msb0;]),
        case::normal(bits![u8, Msb0; 1], Endian::Big, Some(1), 0x1, bits![u8, Msb0;]),
        case::normal(bits![u8, Msb0; 0, 0, 0, 0, 0, 0, 0, 1], Endian::Little, Some(8), 0x1, bits![u8, Msb0;]),
        case::normal(bits![u8, Msb0; 0, 0, 0, 0, 0, 0, 0, 1], Endian::Big, Some(8), 0x1, bits![u8, Msb0;]),
        #[should_panic(expected = "Parse(\"too much data: container of 8 bits cannot hold 9 bits\")")]
        case::normal(bits![u8, Msb0; 1], Endian::Big, Some(9), 0x1, bits![u8, Msb0;]),
    )]
    fn test_bitvec_read(
        bit_slice: &BitSlice<u8, Msb0>,
        endian: Endian,
        bit_size: Option<usize>,
        expected: u8,
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let (amt_read, res_read) = match bit_size {
            Some(bit_size) => u8::read(bit_slice, (endian, BitSize(bit_size))).unwrap(),
            None => u8::read(bit_slice, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);
    }

    #[rstest(input, endian, bit_size, expected, expected_rest,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![u8, Msb0;]),
        case::normal_bits_12_le([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Little, Some(12), 0b1110_1001_0110, bits![u8, Msb0; 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1]),
        case::normal_bits_12_be([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Big, Some(12), 0b1001_0110_1110, bits![u8, Msb0; 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1]),
        case::normal_bit_6([0b1001_0110].as_ref(), Endian::Little, Some(6), 0b1001_01, bits![u8, Msb0; 1, 0,]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(32), 0xFF, bits![u8, Msb0;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(32), 0xFF, bits![u8, Msb0;]),
        #[should_panic(expected = "Parse(\"too much data: container of 32 bits cannot hold 64 bits\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(64), 0xFF, bits![u8, Msb0;]),
    )]
    fn test_bit_read(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (amt_read, res_read) = match bit_size {
            Some(bit_size) => u32::read(bit_slice, (endian, BitSize(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);

        // test both Read &[u8] and Read BitVec
        let res_read = match bit_size {
            Some(bit_size) => {
                u32::from_reader(&mut Container::new(input), (endian, BitSize(bit_size))).unwrap()
            }
            None => u32::from_reader(&mut Container::new(input), endian).unwrap(),
        };
        assert_eq!(expected, res_read);

        let res_read = match bit_size {
            Some(bit_size) => {
                u32::from_reader(&mut Container::new(bit_slice), (endian, BitSize(bit_size)))
                    .unwrap()
            }
            None => u32::from_reader(&mut Container::new(bit_slice), endian).unwrap(),
        };
        assert_eq!(expected, res_read);
    }

    #[rstest(input, endian, byte_size, expected, expected_rest,
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, Some(4), 0xDDCC_BBAA, bits![u8, Msb0;]),
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(4), 0xAABB_CCDD, bits![u8, Msb0;]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, Some(3), 0x00DDCC_BB, bits![u8, Msb0; 1, 0, 1, 0, 1, 0, 1, 0]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(3), 0x00BB_CCDD, bits![u8, Msb0; 1, 0, 1, 0, 1, 0, 1, 0]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(4), 0xFF, bits![u8, Msb0;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(4), 0xFF, bits![u8, Msb0;]),
        #[should_panic(expected = "Parse(\"too much data: container of 32 bits cannot hold 64 bits\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(8), 0xFF, bits![u8, Msb0;]),
    )]
    fn test_byte_read(
        input: &[u8],
        endian: Endian,
        byte_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<u8, Msb0>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (amt_read, res_read) = match byte_size {
            Some(bit_size) => u32::read(bit_slice, (endian, ByteSize(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);

        // test both Read &[u8] and Read BitVec
        let res_read = match byte_size {
            Some(byte_size) => {
                u32::from_reader(&mut Container::new(input), (endian, ByteSize(byte_size))).unwrap()
            }
            None => u32::from_reader(&mut Container::new(input), endian).unwrap(),
        };
        assert_eq!(expected, res_read);

        let res_read = match byte_size {
            Some(byte_size) => u32::from_reader(
                &mut Container::new(bit_slice),
                (endian, ByteSize(byte_size)),
            )
            .unwrap(),
            None => u32::from_reader(&mut Container::new(bit_slice), endian).unwrap(),
        };
        assert_eq!(expected, res_read);
    }

    #[rstest(input, endian, bit_size, expected,
        case::normal_le(0xDDCC_BBAA, Endian::Little, None, vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be(0xDDCC_BBAA, Endian::Big, None, vec![0xDD, 0xCC, 0xBB, 0xAA]),
        case::bit_size_le_smaller(0x03AB, Endian::Little, Some(10), vec![0xAB, 0b11_000000]),
        case::bit_size_be_smaller(0x03AB, Endian::Big, Some(10), vec![0b11_1010_10, 0b11_000000]),
        #[should_panic(expected = "InvalidParam(\"bit size 100 is larger then input 32\")")]
        case::bit_size_le_bigger(0x03AB, Endian::Little, Some(100), vec![0xAB, 0b11_000000]),
    )]
    fn test_bit_write(input: u32, endian: Endian, bit_size: Option<usize>, expected: Vec<u8>) {
        let mut res_write = bitvec![u8, Msb0;];
        match bit_size {
            Some(bit_size) => input
                .write(&mut res_write, (endian, BitSize(bit_size)))
                .unwrap(),
            None => input.write(&mut res_write, endian).unwrap(),
        };
        assert_eq!(expected, res_write.into_vec());
    }

    #[rstest(input, endian, bit_size, expected, expected_rest, expected_write,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![u8, Msb0;], vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<u8, Msb0>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (amt_read, res_read) = match bit_size {
            Some(bit_size) => u32::read(bit_slice, (endian, BitSize(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, bit_slice[amt_read..]);

        let res_read = match bit_size {
            Some(bit_size) => {
                u32::from_reader(&mut Container::new(bit_slice), (endian, BitSize(bit_size)))
                    .unwrap()
            }
            None => u32::from_reader(&mut Container::new(bit_slice), endian).unwrap(),
        };
        assert_eq!(expected, res_read);

        let mut res_write = bitvec![u8, Msb0;];
        match bit_size {
            Some(bit_size) => res_read
                .write(&mut res_write, (endian, BitSize(bit_size)))
                .unwrap(),
            None => res_read.write(&mut res_write, endian).unwrap(),
        };

        assert_eq!(expected_write, res_write.into_vec());
    }

    macro_rules! TestSignExtending {
        ($test_name:ident, $typ:ty) => {
            #[test]
            fn $test_name() {
                let bit_slice = [0b10101_000].view_bits::<Msb0>();

                let (amt_read, res_read) = <$typ>::read(bit_slice, (Endian::Little, BitSize(5))).unwrap();
                assert_eq!(-11, res_read);
                assert_eq!(bits![u8, Msb0; 0, 0, 0], bit_slice[amt_read..]);

                let res_read = <$typ>::from_reader(&mut Container::new(bit_slice), (Endian::Little, BitSize(5))).unwrap();
                assert_eq!(-11, res_read);
            }
        };
    }

    TestSignExtending!(test_sign_extend_i8, i8);
    TestSignExtending!(test_sign_extend_i16, i16);
    TestSignExtending!(test_sign_extend_i32, i32);
    TestSignExtending!(test_sign_extend_i64, i64);
    TestSignExtending!(test_sign_extend_i128, i128);
    TestSignExtending!(test_sign_extend_isize, isize);
}
