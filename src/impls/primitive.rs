use crate::{ctx::*, DekuError, DekuRead, DekuWrite};
use bitvec::prelude::*;
use core::convert::TryInto;

#[cfg(feature = "alloc")]
use alloc::format;

macro_rules! ImplDekuTraits {
    ($typ:ty) => {
        impl DekuRead<'_, (Endian, Size)> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                (endian, size): (Endian, Size),
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let max_type_bits: usize = Size::of::<$typ>().bit_size();
                let bit_size: usize = size.bit_size();

                let input_is_le = endian.is_le();

                if bit_size > max_type_bits {
                    return Err(DekuError::Parse(format!(
                        "too much data: container of {} bits cannot hold {} bits",
                        max_type_bits, bit_size
                    )));
                }

                if input.len() < bit_size {
                    return Err(DekuError::Incomplete(crate::error::NeedSize::new(bit_size)));
                }

                let (bit_slice, rest) = input.split_at(bit_size);

                let pad = 8 * ((bit_slice.len() + 7) / 8) - bit_slice.len();

                let value = if pad == 0 && bit_slice.len() == max_type_bits {
                    // if everything is aligned, just read the value

                    let bytes: &[u8] = bit_slice.as_raw_slice();

                    // Read value
                    if input_is_le {
                        <$typ>::from_le_bytes(bytes.try_into()?)
                    } else {
                        <$typ>::from_be_bytes(bytes.try_into()?)
                    }
                } else {
                    // Create a new BitVec from the slice and pad un-aligned chunks
                    // i.e. [10010110, 1110] -> [10010110, 00001110]
                    let bits: BitVec<Msb0, u8> = {
                        let mut bits = BitVec::with_capacity(bit_slice.len() + pad);

                        // Copy bits to new BitVec
                        bits.extend_from_bitslice(bit_slice);

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
                        for _ in 0..(max_type_bits - bits.len()) {
                            if input_is_le {
                                bits.push(false);
                            } else {
                                bits.insert(0, false);
                            }
                        }

                        bits
                    };

                    let bytes: &[u8] = bits.as_raw_slice();

                    // Read value
                    if input_is_le {
                        <$typ>::from_le_bytes(bytes.try_into()?)
                    } else {
                        <$typ>::from_be_bytes(bytes.try_into()?)
                    }
                };

                Ok((rest, value))
            }
        }

        // Only have `endian`, set `bit_size` to `Size::of::<Type>()`
        impl DekuRead<'_, Endian> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                endian: Endian,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let max_type_bits = Size::of::<$typ>();

                <$typ>::read(input, (endian, max_type_bits))
            }
        }

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuRead<'_, Size> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                bit_size: Size,
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let endian = Endian::default();

                <$typ>::read(input, (endian, bit_size))
            }
        }

        impl DekuRead<'_> for $typ {
            fn read(
                input: &BitSlice<Msb0, u8>,
                _: (),
            ) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                <$typ>::read(input, Endian::default())
            }
        }

        impl DekuWrite<(Endian, Size)> for $typ {
            fn write(
                &self,
                output: &mut BitVec<Msb0, u8>,
                (endian, size): (Endian, Size),
            ) -> Result<(), DekuError> {
                let input = match endian {
                    Endian::Little => self.to_le_bytes(),
                    Endian::Big => self.to_be_bytes(),
                };

                let bit_size: usize = size.bit_size();

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
            fn write(
                &self,
                output: &mut BitVec<Msb0, u8>,
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

        // Only have `bit_size`, set `endian` to `Endian::default`.
        impl DekuWrite<Size> for $typ {
            fn write(
                &self,
                output: &mut BitVec<Msb0, u8>,
                bit_size: Size,
            ) -> Result<(), DekuError> {
                <$typ>::write(self, output, (Endian::default(), bit_size))
            }
        }

        impl DekuWrite for $typ {
            fn write(&self, output: &mut BitVec<Msb0, u8>, _: ()) -> Result<(), DekuError> {
                <$typ>::write(self, output, Endian::default())
            }
        }
    };
}

ImplDekuTraits!(u8);
ImplDekuTraits!(u16);
ImplDekuTraits!(u32);
ImplDekuTraits!(u64);
ImplDekuTraits!(u128);
ImplDekuTraits!(usize);
ImplDekuTraits!(i8);
ImplDekuTraits!(i16);
ImplDekuTraits!(i32);
ImplDekuTraits!(i64);
ImplDekuTraits!(i128);
ImplDekuTraits!(isize);
ImplDekuTraits!(f32);
ImplDekuTraits!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_endian;
    use rstest::rstest;

    static ENDIAN: Endian = Endian::new();

    macro_rules! TestPrimitive {
        ($test_name:ident, $typ:ty, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let input = $input;
                let bit_slice = input.view_bits::<Msb0>();
                let (_rest, res_read) = <$typ>::read(bit_slice, ENDIAN).unwrap();
                assert_eq!($expected, res_read);

                let mut res_write = bitvec![Msb0, u8;];
                res_read.write(&mut res_write, ENDIAN).unwrap();
                assert_eq!(input, res_write.into_vec());
            }
        };
    }

    TestPrimitive!(test_u8, u8, vec![0xAAu8], 0xAAu8);
    TestPrimitive!(
        test_u16,
        u16,
        vec![0xABu8, 0xCD],
        native_endian!(0xCDAB_u16)
    );
    TestPrimitive!(
        test_u32,
        u32,
        vec![0xABu8, 0xCD, 0xEF, 0xBE],
        native_endian!(0xBEEFCDAB_u32)
    );
    TestPrimitive!(
        test_u64,
        u64,
        vec![0xABu8, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD, 0xFE, 0xC0],
        native_endian!(0xC0FECDABBEEFCDAB_u64)
    );
    TestPrimitive!(
        test_u128,
        u128,
        vec![
            0xABu8, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD, 0xFE, 0xC0, 0xAB, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD,
            0xFE, 0xC0
        ],
        native_endian!(0xC0FECDABBEEFCDABC0FECDABBEEFCDAB_u128)
    );
    TestPrimitive!(
        test_usize,
        usize,
        vec![0xABu8, 0xCD, 0xEF, 0xBE, 0xAB, 0xCD, 0xFE, 0xC0],
        if core::mem::size_of::<usize>() == 8 {
            native_endian!(0xC0FECDABBEEFCDAB_usize)
        } else {
            native_endian!(0xBEEFCDAB_usize)
        }
    );
    TestPrimitive!(test_i8, i8, vec![0xFBu8], -5);
    TestPrimitive!(test_i16, i16, vec![0xFDu8, 0xFE], native_endian!(-259_i16));
    TestPrimitive!(
        test_i32,
        i32,
        vec![0x02u8, 0x3F, 0x01, 0xEF],
        native_endian!(-0x10FEC0FE_i32)
    );
    TestPrimitive!(
        test_i64,
        i64,
        vec![0x02u8, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF],
        native_endian!(-0x10FEC0FE10FEC0FE_i64)
    );
    TestPrimitive!(
        test_i128,
        i128,
        vec![
            0x02u8, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF, 0x01, 0x3F,
            0x01, 0xEF
        ],
        native_endian!(-0x10FEC0FE10FEC0FE10FEC0FE10FEC0FE_i128)
    );
    TestPrimitive!(
        test_isize,
        isize,
        vec![0x02u8, 0x3F, 0x01, 0xEF, 0x01, 0x3F, 0x01, 0xEF],
        if core::mem::size_of::<isize>() == 8 {
            native_endian!(-0x10FEC0FE10FEC0FE_isize)
        } else {
            native_endian!(-0x10FEC0FE_isize)
        }
    );
    TestPrimitive!(
        test_f32,
        f32,
        vec![0xA6u8, 0x9B, 0xC4, 0xBB],
        native_endian!(-0.006_f32)
    );
    TestPrimitive!(
        test_f64,
        f64,
        vec![0xFAu8, 0x7E, 0x6A, 0xBC, 0x74, 0x93, 0x78, 0xBF],
        native_endian!(-0.006_f64)
    );

    #[rstest(input, endian, bit_size, expected, expected_rest,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![Msb0, u8;]),
        case::normal_bits_12_le([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Little, Some(12), 0b1110_1001_0110, bits![Msb0, u8; 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1]),
        case::normal_bits_12_be([0b1001_0110, 0b1110_0000, 0xCC, 0xDD ].as_ref(), Endian::Big, Some(12), 0b1001_0110_1110, bits![Msb0, u8; 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1]),
        case::normal_bit_6([0b1001_0110].as_ref(), Endian::Little, Some(6), 0b1001_01, bits![Msb0, u8; 1, 0,]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([].as_ref(), Endian::Little, Some(32), 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected = "Incomplete(NeedSize { bits: 32 })")]
        case::not_enough_data([0xAA, 0xBB].as_ref(), Endian::Little, Some(32), 0xFF, bits![Msb0, u8;]),
        #[should_panic(expected = "Parse(\"too much data: container of 32 bits cannot hold 64 bits\")")]
        case::too_much_data([0xAA, 0xBB, 0xCC, 0xDD, 0xAA, 0xBB, 0xCC, 0xDD].as_ref(), Endian::Little, Some(64), 0xFF, bits![Msb0, u8;]),
    )]
    fn test_bit_read(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => u32::read(bit_slice, (endian, Size::Bits(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };

        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);
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
        let mut res_write = bitvec![Msb0, u8;];
        match bit_size {
            Some(bit_size) => input
                .write(&mut res_write, (endian, Size::Bits(bit_size)))
                .unwrap(),
            None => input.write(&mut res_write, endian).unwrap(),
        };
        assert_eq!(expected, res_write.into_vec());
    }

    #[rstest(input, endian, bit_size, expected, expected_rest, expected_write,
        case::normal([0xDD, 0xCC, 0xBB, 0xAA].as_ref(), Endian::Little, Some(32), 0xAABB_CCDD, bits![Msb0, u8;], vec![0xDD, 0xCC, 0xBB, 0xAA]),
    )]
    fn test_bit_read_write(
        input: &[u8],
        endian: Endian,
        bit_size: Option<usize>,
        expected: u32,
        expected_rest: &BitSlice<Msb0, u8>,
        expected_write: Vec<u8>,
    ) {
        let bit_slice = input.view_bits::<Msb0>();

        let (rest, res_read) = match bit_size {
            Some(bit_size) => u32::read(bit_slice, (endian, Size::Bits(bit_size))).unwrap(),
            None => u32::read(bit_slice, endian).unwrap(),
        };
        assert_eq!(expected, res_read);
        assert_eq!(expected_rest, rest);

        let mut res_write = bitvec![Msb0, u8;];
        match bit_size {
            Some(bit_size) => res_read
                .write(&mut res_write, (endian, Size::Bits(bit_size)))
                .unwrap(),
            None => res_read.write(&mut res_write, endian).unwrap(),
        };

        assert_eq!(expected_write, res_write.into_vec());
    }
}
