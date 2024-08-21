use alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use alloc::format;
use core::convert::TryInto;

use bitvec::prelude::*;
use no_std_io::io::{Read, Write};

use crate::ctx::*;
use crate::reader::{Reader, ReaderRet};
use crate::writer::Writer;
use crate::{DekuError, DekuReader, DekuWriter};

/// "Read" trait: read bits and construct type
trait DekuRead<'a, Ctx = ()> {
    /// Read bits and construct type
    /// * **input** - Input as bits
    /// * **ctx** - A context required by context-sensitive reading. A unit type `()` means no context
    /// needed.
    ///
    /// Returns the amount of bits read after parsing in addition to Self.
    ///
    /// NOTE: since this is only used internally by primitive types, we don't need to verify the
    /// size of BitSize or ByteSize to check if they fit in the requested container size
    /// (size_of::<type>()).
    fn read(
        input: &'a crate::bitvec::BitSlice<u8, crate::bitvec::Msb0>,
        ctx: Ctx,
    ) -> Result<(usize, Self), DekuError>
    where
        Self: Sized;
}

// specialize u8 for ByteSize
impl DekuRead<'_, (Endian, ByteSize)> for u8 {
    /// Ignore endian and byte_size, as this is a `u8`
    #[inline]
    fn read(
        input: &BitSlice<u8, Msb0>,
        (_, _): (Endian, ByteSize),
    ) -> Result<(usize, Self), DekuError> {
        const MAX_TYPE_BITS: usize = BitSize::of::<u8>().0;

        // PANIC: We already check that input.len() < bit_size above, so no panic will happen
        let value = input[..MAX_TYPE_BITS].load::<u8>();
        Ok((MAX_TYPE_BITS, value))
    }
}

impl DekuReader<'_, (Endian, ByteSize, Order)> for u8 {
    /// Ignore endian, as this is a `u8`
    #[inline(always)]
    fn from_reader_with_ctx<R: Read>(
        reader: &mut Reader<R>,
        (endian, size, order): (Endian, ByteSize, Order),
    ) -> Result<u8, DekuError> {
        const MAX_TYPE_BYTES: usize = core::mem::size_of::<u8>();
        let mut buf = [0; MAX_TYPE_BYTES];
        let ret = reader.read_bytes_const::<MAX_TYPE_BYTES>(&mut buf, order)?;
        let a = match ret {
            ReaderRet::Bytes => <u8>::from_be_bytes(buf),
            ReaderRet::Bits(bits) => {
                let Some(bits) = bits else {
                    return Err(DekuError::Parse(Cow::from("no bits read from reader")));
                };
                let a = <u8>::read(&bits, (endian, size))?;
                a.1
            }
        };
        Ok(a)
    }
}

impl DekuWriter<(Endian, BitSize)> for u8 {
    /// Ignore endian, as this is a `u8`
    #[inline(always)]
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        (_, bit_size): (Endian, BitSize),
    ) -> Result<(), DekuError> {
        // endian doens't matter
        <u8>::to_writer(self, writer, (Endian::Big, bit_size, Order::Msb0))
    }
}

impl DekuWriter<(Endian, BitSize, Order)> for u8 {
    /// Ignore endian, as this is a `u8`
    #[inline(always)]
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        (_, size, order): (Endian, BitSize, Order),
    ) -> Result<(), DekuError> {
        let input = self.to_le_bytes();

        let bit_size: usize = size.0;

        let input_bits = input.view_bits::<Msb0>();

        if bit_size > input_bits.len() {
            return Err(DekuError::InvalidParam(Cow::from(format!(
                "bit size {} is larger than input {}",
                bit_size,
                input_bits.len()
            ))));
        }

        // Check for extra bits before sending into writer
        const MAX_TYPE_BITS: usize = BitSize::of::<u8>().0;
        if let Some(first) = input_bits.first_one() {
            let max = MAX_TYPE_BITS - bit_size;
            if max > first {
                return Err(DekuError::InvalidParam(Cow::from(format!(
                    "bit size {} of input is larger than bit requested size {}",
                    MAX_TYPE_BITS - first,
                    bit_size,
                ))));
            }
        }
        writer.write_bits_order(&input_bits[input_bits.len() - bit_size..], order)?;
        Ok(())
    }
}

impl DekuWriter<(Endian, ByteSize, Order)> for u8 {
    /// Ignore endian and byte_size, as this is a `u8`
    #[inline(always)]
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        (_, _, _): (Endian, ByteSize, Order),
    ) -> Result<(), DekuError> {
        let input = self.to_le_bytes();
        writer.write_bytes(&input)?;
        Ok(())
    }
}

impl DekuWriter<(Endian, ByteSize)> for u8 {
    /// Ignore endian and byte_size, as this is a `u8`
    #[inline(always)]
    fn to_writer<W: Write>(
        &self,
        writer: &mut Writer<W>,
        (_, _): (Endian, ByteSize),
    ) -> Result<(), DekuError> {
        let input = self.to_le_bytes();
        writer.write_bytes(&input)?;
        Ok(())
    }
}

macro_rules! ImplDekuReadBits {
    ($typ:ty, $inner:ty) => {
        impl DekuRead<'_, (Endian, BitSize, Order)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size, order): (Endian, BitSize, Order),
            ) -> Result<(usize, Self), DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size: usize = size.0;

                let input_is_le = endian.is_le();

                // PANIC: We already check that input.len() < bit_size above, so no panic will happen
                let bit_slice = &input;

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

                // if read from Lsb order and it's escpecially cursed since its not just within one byte...
                // read_bits returned: [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1]
                //                     | second     |    first            |
                // we want to read from right to left when lsb (without using BitVec BitFields)
                //
                // Turning this into [0x23, 0x01] (then appending till type size)
                if order == Order::Lsb0 && bit_slice.len() > 8 {
                    let mut bits = BitVec::<u8, Msb0>::with_capacity(bit_slice.len() + pad);

                    bits.extend_from_bitslice(&bit_slice);

                    for _ in 0..pad {
                        bits.insert(0, false);
                    }

                    let mut buf = vec![];
                    let mut n = bits.len() - 8;
                    while let Some(slice) = bits.get(n..n + 8) {
                        let a: u8 = slice.load_be();
                        buf.push(a);
                        if n < 8 {
                            break;
                        }
                        n -= 8;
                    }

                    // Pad up-to size of type
                    for _ in 0..core::mem::size_of::<$typ>() - buf.len() {
                        buf.push(0x00);
                    }

                    // Read value
                    let value = if input_is_le {
                        <$typ>::from_le_bytes(buf.try_into().unwrap())
                    } else {
                        <$typ>::from_be_bytes(buf.try_into().unwrap())
                    };

                    Ok((bit_size, value))
                } else {
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
        }

        impl DekuRead<'_, (Endian, BitSize)> for $typ {
            #[inline(never)]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size): (Endian, BitSize),
            ) -> Result<(usize, Self), DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size: usize = size.0;

                let input_is_le = endian.is_le();

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
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, size): (Endian, BitSize),
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                if size.0 > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(Cow::from(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {} bits",
                        size.0
                    ))));
                }
                let bits = reader.read_bits(size.0, Order::Msb0)?;
                let Some(bits) = bits else {
                    return Err(DekuError::Parse(Cow::from("no bits read from reader")));
                };
                let a = <$typ>::read(&bits, (endian, size))?;
                Ok(a.1)
            }
        }

        impl DekuReader<'_, (Endian, BitSize, Order)> for $typ {
            #[inline]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, size, order): (Endian, BitSize, Order),
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                if size.0 > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(Cow::from(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {} bits",
                        size.0
                    ))));
                }
                let bits = reader.read_bits(size.0, order)?;
                let Some(bits) = bits else {
                    return Err(DekuError::Parse(Cow::from(format!(
                        "no bits read from reader",
                    ))));
                };
                let a = <$typ>::read(&bits, (endian, size, order))?;
                Ok(a.1)
            }
        }
    };
}

macro_rules! ImplDekuReadBytes {
    ($typ:ty, $inner:ty) => {
        /// Ignore order
        impl DekuRead<'_, (Endian, ByteSize, Order)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size, _order): (Endian, ByteSize, Order),
            ) -> Result<(usize, Self), DekuError> {
                <$typ as DekuRead<'_, (Endian, ByteSize)>>::read(input, (endian, size))
            }
        }

        impl DekuRead<'_, (Endian, ByteSize)> for $typ {
            #[inline(never)]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<(usize, Self), DekuError> {
                let bit_size: usize = size.0 * 8;

                let input_is_le = endian.is_le();

                let bit_slice = &input[..bit_size];

                let bytes = bit_slice.domain().region().unwrap().1;
                let value = if input_is_le {
                    <$typ>::from_le_bytes(bytes.try_into()?)
                } else {
                    <$typ>::from_be_bytes(bytes.try_into()?)
                };

                Ok((bit_size, value))
            }
        }

        impl DekuReader<'_, (Endian, ByteSize, Order)> for $typ {
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, size, order): (Endian, ByteSize, Order),
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BYTES: usize = core::mem::size_of::<$typ>();
                if size.0 > MAX_TYPE_BYTES {
                    return Err(DekuError::Parse(Cow::from(format!(
                        "too much data: container of {MAX_TYPE_BYTES} bytes cannot hold {} bytes",
                        size.0
                    ))));
                }
                let mut buf = [0; MAX_TYPE_BYTES];
                let ret = reader.read_bytes(size.0, &mut buf, order)?;
                let a = match ret {
                    ReaderRet::Bytes => {
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
                    ReaderRet::Bits(Some(bits)) => {
                        let a = <$typ>::read(&bits, (endian, size))?;
                        a.1
                    }
                    ReaderRet::Bits(None) => {
                        return Err(DekuError::Parse(Cow::from("no bits read from reader")));
                    }
                };
                Ok(a)
            }
        }
    };
}

macro_rules! ImplDekuReadSignExtend {
    ($typ:ty, $inner:ty) => {
        // Ignore Order, send back
        impl DekuRead<'_, (Endian, ByteSize, Order)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size, _order): (Endian, ByteSize, Order),
            ) -> Result<(usize, Self), DekuError> {
                <$typ as DekuRead<'_, (Endian, ByteSize)>>::read(input, (endian, size))
            }
        }

        impl DekuRead<'_, (Endian, BitSize, Order)> for $typ {
            #[inline]
            fn read(
                input: &BitSlice<u8, Msb0>,
                (endian, size, order): (Endian, BitSize, Order),
            ) -> Result<(usize, Self), DekuError> {
                let (amt_read, value) = <$inner as DekuRead<'_, (Endian, BitSize, Order)>>::read(
                    input,
                    (endian, size, order),
                )?;

                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size = size.0;
                let shift = MAX_TYPE_BITS - bit_size;
                let value = (value as $typ) << shift >> shift;
                Ok((amt_read, value))
            }
        }

        impl DekuRead<'_, (Endian, ByteSize)> for $typ {
            #[inline(never)]
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

        impl DekuRead<'_, (Endian, BitSize)> for $typ {
            #[inline(never)]
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
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, size): (Endian, BitSize),
            ) -> Result<$typ, DekuError> {
                <$typ>::from_reader_with_ctx(reader, (endian, size, Order::Msb0))
            }
        }

        impl DekuReader<'_, (Endian, BitSize, Order)> for $typ {
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, size, order): (Endian, BitSize, Order),
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                if size.0 > MAX_TYPE_BITS {
                    return Err(DekuError::Parse(Cow::from(format!(
                        "too much data: container of {MAX_TYPE_BITS} bits cannot hold {} bits",
                        size.0
                    ))));
                }
                let bits = reader.read_bits(size.0, order)?;
                let Some(bits) = bits else {
                    return Err(DekuError::Parse(Cow::from("no bits read from reader")));
                };
                let a = <$typ>::read(&bits, (endian, size, Order::Msb0))?;
                Ok(a.1)
            }
        }

        // TODO: Remove
        impl DekuReader<'_, (Endian, ByteSize, Order)> for $typ {
            #[inline]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, size, order): (Endian, ByteSize, Order),
            ) -> Result<$typ, DekuError> {
                let mut buf = [0; core::mem::size_of::<$typ>()];
                let ret = reader.read_bytes(size.0, &mut buf, order)?;
                let a = match ret {
                    ReaderRet::Bits(bits) => {
                        let Some(bits) = bits else {
                            return Err(DekuError::Parse(Cow::from(
                                "no bits read from reader".to_string(),
                            )));
                        };
                        let a = <$typ>::read(&bits, (endian, size))?;
                        a.1
                    }
                    ReaderRet::Bytes => {
                        if endian.is_le() {
                            <$typ>::from_le_bytes(buf.try_into()?)
                        } else {
                            <$typ>::from_be_bytes(buf.try_into()?)
                        }
                    }
                };

                const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                let bit_size = size.0 * 8;
                let shift = MAX_TYPE_BITS - bit_size;
                let value = (a as $typ) << shift >> shift;
                Ok(value)
            }
        }
    };
}

macro_rules! ForwardDekuRead {
    ($typ:ty) => {
        impl DekuReader<'_, (Endian, Order)> for $typ {
            #[inline]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, order): (Endian, Order),
            ) -> Result<$typ, DekuError> {
                let byte_size = core::mem::size_of::<$typ>();

                <$typ>::from_reader_with_ctx(reader, (endian, ByteSize(byte_size), order))
            }
        }

        impl DekuReader<'_, (Endian, ByteSize)> for $typ {
            #[inline]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (endian, byte_size): (Endian, ByteSize),
            ) -> Result<$typ, DekuError> {
                <$typ>::from_reader_with_ctx(reader, (endian, byte_size, Order::Msb0))
            }
        }

        // Only have `endian`, specialize and use read_bytes_const
        impl DekuReader<'_, Endian> for $typ {
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                endian: Endian,
            ) -> Result<$typ, DekuError> {
                const MAX_TYPE_BYTES: usize = core::mem::size_of::<$typ>();
                let mut buf = [0; MAX_TYPE_BYTES];
                let ret = reader.read_bytes_const::<MAX_TYPE_BYTES>(&mut buf, Order::Msb0)?;
                let a = match ret {
                    ReaderRet::Bytes => {
                        if endian.is_le() {
                            <$typ>::from_le_bytes(buf)
                        } else {
                            <$typ>::from_be_bytes(buf)
                        }
                    }
                    ReaderRet::Bits(Some(bits)) => {
                        let a = <$typ>::read(&bits, (endian, ByteSize(MAX_TYPE_BYTES)))?;
                        a.1
                    }
                    ReaderRet::Bits(None) => {
                        return Err(DekuError::Parse(Cow::from("no bits read from reader")));
                    }
                };
                Ok(a)
            }
        }

        // Only have `byte_size`, set `endian` to `Endian::default`.
        impl DekuReader<'_, ByteSize> for $typ {
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                byte_size: ByteSize,
            ) -> Result<$typ, DekuError> {
                let endian = Endian::default();

                let a = <$typ>::from_reader_with_ctx(reader, (endian, byte_size))?;
                Ok(a)
            }
        }

        //// Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuReader<'_, BitSize> for $typ {
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                bit_size: BitSize,
            ) -> Result<$typ, DekuError> {
                let endian = Endian::default();

                if (bit_size.0 % 8) == 0 {
                    <$typ>::from_reader_with_ctx(reader, (endian, ByteSize(bit_size.0 / 8)))
                } else {
                    <$typ>::from_reader_with_ctx(reader, (endian, bit_size))
                }
            }
        }

        //// Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuReader<'_, (BitSize, Order)> for $typ {
            #[inline]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                (bit_size, order): (BitSize, Order),
            ) -> Result<$typ, DekuError> {
                let endian = Endian::default();

                if (bit_size.0 % 8) == 0 {
                    <$typ>::from_reader_with_ctx(reader, (endian, ByteSize(bit_size.0 / 8), order))
                } else {
                    <$typ>::from_reader_with_ctx(reader, (endian, bit_size, order))
                }
            }
        }

        impl DekuReader<'_, Order> for $typ {
            #[inline]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                order: Order,
            ) -> Result<$typ, DekuError> {
                <$typ>::from_reader_with_ctx(reader, (Endian::default(), order))
            }
        }

        impl DekuReader<'_> for $typ {
            #[inline(always)]
            fn from_reader_with_ctx<R: Read>(
                reader: &mut Reader<R>,
                _: (),
            ) -> Result<$typ, DekuError> {
                <$typ>::from_reader_with_ctx(reader, Endian::default())
            }
        }
    };
}

macro_rules! ImplDekuWrite {
    ($typ:ty) => {
            impl DekuWriter<(Endian, BitSize, Order)> for $typ {
            #[inline]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                (endian, size, order): (Endian, BitSize, Order),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                let bit_size: usize = size.0;

                let input_bits = input.view_bits::<Msb0>();

                if bit_size > input_bits.len() {
                    return Err(DekuError::InvalidParam(Cow::from(format!(
                        "bit size {} is larger then input {}",
                        bit_size,
                        input_bits.len()
                    ))));
                }

                match (endian, order) {
                    (Endian::Little, Order::Lsb0)
                    | (Endian::Little, Order::Msb0)
                    | (Endian::Big, Order::Lsb0) => {
                        let mut remaining_bits = bit_size;
                        for chunk in input_bits.chunks(8) {
                            if chunk.len() > remaining_bits {
                                writer.write_bits_order(
                                    &chunk[chunk.len() - remaining_bits..],
                                    order,
                                )?;
                                break;
                            } else {
                                writer.write_bits_order(&chunk, order)?;
                            }
                            remaining_bits -= chunk.len();
                        }
                    }
                    (Endian::Big, Order::Msb0) => {
                        // big endian
                        // Example read 10 bits u32 [0xAB, 0b11_000000]
                        // => [00000000, 00000000, 00000010, 10101111]
                        writer.write_bits_order(
                            &input_bits[input_bits.len() - bit_size..],
                            Order::Msb0,
                        )?;
                    }
                }

                Ok(())
            }
        }

        impl DekuWriter<(Endian, BitSize)> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                (endian, size): (Endian, BitSize),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                let bit_size: usize = size.0;

                let input_bits = input.view_bits::<Msb0>();

                if bit_size > input_bits.len() {
                    return Err(DekuError::InvalidParam(Cow::from(format!(
                        "bit size {} is larger than input {}",
                        bit_size,
                        input_bits.len()
                    ))));
                }

                if matches!(endian, Endian::Little) {
                    // Check if this is a value that will fit inside the required bits, if
                    // not, throw an error
                    let input_bits_lsb = input.view_bits::<Lsb0>();
                    if let Some(last) = input_bits_lsb.last_one() {
                        let last = last + 1;
                        let max = bit_size;
                        if last > max {
                            return Err(DekuError::InvalidParam(Cow::from(format!(
                                "bit size {last} of input is larger than bit requested size {bit_size}",
                            ))));
                        }
                    }

                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [10101011, 00000011, 00000000, 00000000]
                    let mut remaining_bits = bit_size;
                    for chunk in input_bits.chunks(8) {
                        if chunk.len() > remaining_bits {
                            writer.write_bits(&chunk[chunk.len() - remaining_bits..])?;
                            break;
                        } else {
                            writer.write_bits(&chunk)?;
                        }
                        remaining_bits -= chunk.len();
                    }
                } else {
                    const MAX_TYPE_BITS: usize = BitSize::of::<$typ>().0;
                    // Check for extra bits before sending into writer
                    if let Some(first) = input_bits.first_one() {
                        let max = (MAX_TYPE_BITS - bit_size);
                        if max > first {
                            return Err(DekuError::InvalidParam(Cow::from(format!(
                                "bit size {} of input is larger than bit requested size {}",
                                MAX_TYPE_BITS - first,
                                bit_size,
                            ))));
                        }
                    }
                    // Example read 10 bits u32 [0xAB, 0b11_000000]
                    // => [00000000, 00000000, 00000010, 10101111]
                    writer.write_bits(&input_bits[input_bits.len() - bit_size..])?;
                }
                Ok(())
            }
        }

        impl DekuWriter<(Endian, ByteSize)> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                (endian, size): (Endian, ByteSize),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                const TYPE_SIZE: usize = core::mem::size_of::<$typ>();
                if size.0 > TYPE_SIZE {
                    return Err(DekuError::InvalidParam(Cow::from(format!(
                        "byte size {} is larger then input {}",
                        size.0, TYPE_SIZE
                    ))));
                }

                let input = if matches!(endian, Endian::Big) {
                    &input[TYPE_SIZE - size.0 as usize..]
                } else {
                    &input[..size.0 as usize]
                };

                writer.write_bytes(&input)?;
                Ok(())
            }
        }

        /// When using Endian and ByteSize, Order is not used
        impl DekuWriter<(Endian, ByteSize, Order)> for $typ {
            #[inline]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                (endian, size, _order): (Endian, ByteSize, Order),
            ) -> Result<(), DekuError> {
                <$typ>::to_writer(self, writer, (endian, size))
            }
        }
    };
}

macro_rules! ImplDekuWriteOnlyEndian {
    ($typ:ty) => {
        impl DekuWriter<Endian> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                endian: Endian,
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };
                writer.write_bytes(&input)?;
                Ok(())
            }
        }
    };
}

macro_rules! ForwardDekuWrite {
    ($typ:ty) => {
        impl DekuWriter<(BitSize, Order)> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                (bit_size, order): (BitSize, Order),
            ) -> Result<(), DekuError> {
                <$typ>::to_writer(self, writer, (Endian::default(), bit_size, order))
            }
        }

        impl DekuWriter<(Endian, Order)> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                (endian, order): (Endian, Order),
            ) -> Result<(), DekuError> {
                let byte_size = core::mem::size_of::<$typ>();
                <$typ>::to_writer(self, writer, (endian, ByteSize(byte_size), order))
            }
        }

        impl DekuWriter<BitSize> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                bit_size: BitSize,
            ) -> Result<(), DekuError> {
                <$typ>::to_writer(self, writer, (Endian::default(), bit_size))
            }
        }

        impl DekuWriter<ByteSize> for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                byte_size: ByteSize,
            ) -> Result<(), DekuError> {
                <$typ>::to_writer(self, writer, (Endian::default(), byte_size))
            }
        }

        impl DekuWriter<Order> for $typ {
            #[inline]
            fn to_writer<W: Write>(
                &self,
                writer: &mut Writer<W>,
                order: Order,
            ) -> Result<(), DekuError> {
                <$typ>::to_writer(self, writer, (Endian::default(), order))
            }
        }

        impl DekuWriter for $typ {
            #[inline(always)]
            fn to_writer<W: Write>(&self, writer: &mut Writer<W>, _: ()) -> Result<(), DekuError> {
                <$typ>::to_writer(self, writer, Endian::default())
            }
        }
    };
}
macro_rules! ImplDekuTraitsBytes {
    ($typ:ty) => {
        ImplDekuReadBytes!($typ, $typ);
        ImplDekuWrite!($typ);
    };
    ($typ:ty, $inner:ty) => {
        ImplDekuReadBytes!($typ, $inner);
    };
}

macro_rules! ImplDekuTraits {
    ($typ:ty) => {
        ImplDekuReadBits!($typ, $typ);
        ForwardDekuRead!($typ);

        ImplDekuWriteOnlyEndian!($typ);
        ForwardDekuWrite!($typ);
    };
    ($typ:ty, $inner:ty) => {
        ImplDekuReadBits!($typ, $inner);
        ForwardDekuRead!($typ);

        ImplDekuWrite!($typ);
        ImplDekuWriteOnlyEndian!($typ);
        ForwardDekuWrite!($typ);
    };
}

macro_rules! ImplDekuTraitsSignExtend {
    ($typ:ty, $inner:ty) => {
        ImplDekuReadSignExtend!($typ, $inner);
        ForwardDekuRead!($typ);

        ImplDekuWrite!($typ);
        ImplDekuWriteOnlyEndian!($typ);
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
    use crate::{native_endian, reader::Reader};

    static ENDIAN: Endian = Endian::new();

    macro_rules! TestPrimitive {
        ($test_name:ident, $typ:ty, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let mut r = std::io::Cursor::new($input);
                let mut reader = Reader::new(&mut r);
                let res_read = <$typ>::from_reader_with_ctx(&mut reader, ENDIAN).unwrap();
                assert_eq!($expected, res_read);

                let mut writer = Writer::new(vec![]);
                res_read.to_writer(&mut writer, ENDIAN).unwrap();
                assert_eq!($input, writer.inner);
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

    #[rstest(input, endian, bit_size, expected, expected_rest_bits, expected_rest_bytes,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![u8, Msb0;], &[]),
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, Some(32), 0xDDCC_BBAA, bits![u8, Msb0;], &[]),
        case::normal_bits_12_le([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Little, Some(12), 0b1110_1001_0110, bits![u8, Msb0; 0, 0, 0, 0], &[0xcc, 0xdd]),
        case::normal_bits_12_be([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Big, Some(12), 0b1001_0110_1110, bits![u8, Msb0; 0, 0, 0, 0], &[0xcc, 0xdd]),
        case::normal_bit_6([0b1001_0110].as_ref(), Endian::Little, Some(6), 0b1001_01, bits![u8, Msb0; 1, 0,], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(32), 0xFF, bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(32), 0xFF, bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 32 bits cannot hold 64 bits\")")] // This will end up in ByteSize b/c 64 % 8 == 0
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(64), 0xFF, bits![u8, Msb0;], &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 32 bits cannot hold 63 bits\")")] // This will end up staying BitSize
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(63), 0xFF, bits![u8, Msb0;], &[]),
    )]
    fn test_bit_read(
        mut input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest_bits: &BitSlice<u8, Msb0>,
        expected_rest_bytes: &[u8],
    ) {
        // test both Read &[u8] and Read BitVec
        let mut reader = Reader::new(&mut input);
        let res_read = match bit_size {
            Some(bit_size) => {
                u32::from_reader_with_ctx(&mut reader, (endian, BitSize(bit_size))).unwrap()
            }
            None => u32::from_reader_with_ctx(&mut reader, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(
            reader.rest(),
            expected_rest_bits.iter().by_vals().collect::<Vec<bool>>()
        );
        let mut buf = vec![];
        input.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[rstest(input, endian, byte_size, expected, expected_rest_bytes,
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, Some(4), 0xDDCC_BBAA, &[]),
        case::normal_le([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(4), 0xAABB_CCDD, &[]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Big, Some(3), 0x00DDCC_BB, &[0xaa]),
        case::normal_be([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(3), 0x00BB_CCDD, &[0xaa]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(4), 0xFF, &[]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(4), 0xFF, &[]),
        #[should_panic(expected = "Parse(\"too much data: container of 4 bytes cannot hold 8 bytes\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(8), 0xFF, &[]),
    )]
    fn test_byte_read(
        mut input: &[u8],
        endian: Endian,
        byte_size: Option<usize>,
        expected: u32,
        expected_rest_bytes: &[u8],
    ) {
        let mut bit_slice = input.view_bits::<Msb0>();

        // test both Read &[u8] and Read BitVec
        let mut reader = Reader::new(&mut input);
        let res_read = match byte_size {
            Some(byte_size) => {
                u32::from_reader_with_ctx(&mut reader, (endian, ByteSize(byte_size))).unwrap()
            }
            None => u32::from_reader_with_ctx(&mut reader, endian).unwrap(),
        };
        assert_eq!(expected, res_read);

        let mut reader = Reader::new(&mut bit_slice);
        let res_read = match byte_size {
            Some(byte_size) => {
                u32::from_reader_with_ctx(&mut reader, (endian, ByteSize(byte_size))).unwrap()
            }
            None => u32::from_reader_with_ctx(&mut reader, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        let mut buf = vec![];
        input.read_to_end(&mut buf).unwrap();
        assert_eq!(expected_rest_bytes, buf);
    }

    #[rstest(input, endian, bit_size, expected, expected_leftover,
        case::normal_le(0xDDCC_BBAA, Endian::Little, None, vec![0xAA, 0xBB, 0xCC, 0xDD], vec![]),
        case::normal_be(0xDDCC_BBAA, Endian::Big, None, vec![0xDD, 0xCC, 0xBB, 0xAA], vec![]),
        case::bit_size_be_smaller(0x03AB, Endian::Big, Some(10), vec![0b11_1010_10], vec![true, true]),
        #[should_panic(expected = "InvalidParam(\"bit size 100 is larger than input 32\")")]
        case::bit_size_le_bigger(0x03AB, Endian::Little, Some(100), vec![0xAB, 0b11_000000], vec![true, true]),
        #[should_panic(expected = "InvalidParam(\"bit size 10 of input is larger than bit requested size 5\")")]
        case::bit_size_larger(0x03AB, Endian::Little, Some(5), vec![], vec![]),
    )]
    fn test_bit_writer(
        input: u32,
        endian: Endian,
        bit_size: Option<usize>,
        expected: Vec<u8>,
        expected_leftover: Vec<bool>,
    ) {
        let mut writer = Writer::new(vec![]);
        match bit_size {
            Some(bit_size) => input
                .to_writer(&mut writer, (endian, BitSize(bit_size)))
                .unwrap(),
            None => input.to_writer(&mut writer, endian).unwrap(),
        };
        assert_eq!(expected_leftover, writer.rest());
        assert_eq!(expected, writer.inner);
    }

    #[rstest(input, endian, byte_size, expected,
        case::normal_le(0xDDCC_BBAA, Endian::Little, None, vec![0xAA, 0xBB, 0xCC, 0xDD]),
        case::normal_be(0xDDCC_BBAA, Endian::Big, None, vec![0xDD, 0xCC, 0xBB, 0xAA]),
        case::byte_size_be_smaller(0x00ffABAA, Endian::Big, Some(2), vec![0xab, 0xaa]),
        #[should_panic(expected = "InvalidParam(\"byte size 10 is larger then input 4\")")]
        case::byte_size_le_bigger(0x03AB, Endian::Little, Some(10), vec![0xAB, 0b11_000000]),
    )]
    fn test_byte_writer(input: u32, endian: Endian, byte_size: Option<usize>, expected: Vec<u8>) {
        let mut writer = Writer::new(vec![]);
        match byte_size {
            Some(byte_size) => input
                .to_writer(&mut writer, (endian, ByteSize(byte_size)))
                .unwrap(),
            None => input.to_writer(&mut writer, endian).unwrap(),
        };
        assert_hex::assert_eq_hex!(expected, writer.inner);
    }

    #[rstest(input, endian, bit_size, expected, expected_write,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_write: Vec<u8>,
    ) {
        let mut bit_slice = input.view_bits::<Msb0>();

        let mut reader = Reader::new(&mut bit_slice);
        let res_read = match bit_size {
            Some(bit_size) => {
                u32::from_reader_with_ctx(&mut reader, (endian, BitSize(bit_size))).unwrap()
            }
            None => u32::from_reader_with_ctx(&mut reader, endian).unwrap(),
        };
        assert_eq!(expected, res_read);

        let mut writer = Writer::new(vec![]);
        match bit_size {
            Some(bit_size) => res_read
                .to_writer(&mut writer, (endian, BitSize(bit_size)))
                .unwrap(),
            None => res_read.to_writer(&mut writer, endian).unwrap(),
        };
        assert_hex::assert_eq_hex!(expected_write, writer.inner);
    }

    macro_rules! TestSignExtending {
        ($test_name:ident, $typ:ty) => {
            #[test]
            fn $test_name() {
                let mut slice = [0b10101_000].as_slice();
                let mut reader = Reader::new(&mut slice);
                let res_read =
                    <$typ>::from_reader_with_ctx(&mut reader, (Endian::Little, BitSize(5)))
                        .unwrap();
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

    macro_rules! TestSignExtendingPanic {
        ($test_name:ident, $typ:ty, $size:expr) => {
            #[test]
            fn $test_name() {
                let mut slice = [0b10101_000].as_slice();
                let mut reader = Reader::new(&mut slice);
                let res_read =
                    <$typ>::from_reader_with_ctx(&mut reader, (Endian::Little, BitSize($size + 1)));
                assert_eq!(
                    DekuError::Parse(Cow::from(format!(
                        "too much data: container of {} bits cannot hold {} bits",
                        $size,
                        $size + 1
                    ))),
                    res_read.err().unwrap()
                );
            }
        };
    }

    TestSignExtendingPanic!(test_sign_extend_i8_panic, i8, 8);
    TestSignExtendingPanic!(test_sign_extend_i16_panic, i16, 16);
    TestSignExtendingPanic!(test_sign_extend_i32_panic, i32, 32);
    TestSignExtendingPanic!(test_sign_extend_i64_panic, i64, 64);
    TestSignExtendingPanic!(test_sign_extend_i128_panic, i128, 128);
}
