#[cfg(test)]
mod tests {
    use deku::prelude::*;
    use hex_literal::hex;
    use rstest::rstest;
    use std::convert::{TryFrom, TryInto};

    pub mod samples {
        use super::*;

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct DoubleNestedDeku {
            pub data: u16,
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct NestedDeku {
            #[deku(bits = "6")]
            pub nest_a: u8,
            #[deku(bits = "2")]
            pub nest_b: u8,

            pub inner: DoubleNestedDeku,
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct UnNamedDeku(
            pub u8,
            #[deku(bits = "2")] pub u8,
            #[deku(bits = "6")] pub u8,
            #[deku(bytes = "2")] pub u16,
            #[deku(endian = "big")] pub u16,
            pub NestedDeku,
            pub u8,
            #[deku(len = "6")] pub Vec<u8>,
        );

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct NamedDeku {
            pub field_a: u8,
            #[deku(bits = "2")]
            pub field_b: u8,
            #[deku(bits = "6")]
            pub field_c: u8,
            #[deku(bytes = "2")]
            pub field_d: u16,
            #[deku(endian = "big")]
            pub field_e: u16,
            pub field_f: NestedDeku,
            pub vec_len: u8,
            #[deku(len = "vec_len")]
            pub vec_data: Vec<u8>,
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        #[deku(id_type = "u8")]
        pub enum EnumDeku {
            #[deku(id = "1")]
            VarA(u8),
            #[deku(id = "2")]
            VarB(#[deku(bits = 4)] u8, #[deku(bits = 4)] u8),
            #[deku(id = "3")]
            VarC {
                field_a: u8,
                #[deku(len = "field_a")]
                field_b: Vec<u8>,
            },
            #[deku(id = "4")]
            VarD(u8, #[deku(len = "0")] Vec<u8>),
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct VecCountDeku {
            pub count: u8,
            #[deku(len = "count")]
            pub vec_data: Vec<u8>,
        }

        #[derive(PartialEq, Debug, DekuRead)]
        pub struct MapDeku {
            #[deku(map = "|field: u8| -> Result<_, DekuError> { Ok(field.to_string()) }")]
            pub field_a: String,
            #[deku(map = "MapDeku::map_field_b")]
            pub field_b: String,
        }

        impl MapDeku {
            fn map_field_b(field_b: u8) -> Result<String, DekuError> {
                Ok(field_b.to_string())
            }
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct ReaderWriterDeku {
            #[deku(
                reader = "ReaderWriterDeku::read(rest, input_is_le, field_bits)",
                writer = "ReaderWriterDeku::write(self.field_a, output_is_le, field_bits)"
            )]
            pub field_a: u8,
        }

        impl ReaderWriterDeku {
            fn read(
                rest: &BitSlice<Msb0, u8>,
                input_is_le: bool,
                bit_size: Option<usize>,
            ) -> Result<(&BitSlice<Msb0, u8>, u8), DekuError> {
                let (rest, value) = u8::read(rest, input_is_le, bit_size, None)?;
                Ok((rest, value + 1))
            }

            fn write(
                field_a: u8,
                output_is_le: bool,
                bit_size: Option<usize>,
            ) -> Result<BitVec<Msb0, u8>, DekuError> {
                let value = field_a - 1;
                value.write(output_is_le, bit_size)
            }
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        pub struct GenericStructDeku<T: deku::BitsWriter + deku::BitsReader>
        where
            T: deku::BitsWriter + deku::BitsReader,
        {
            pub field_a: T,
        }

        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        #[deku(id_type = "u8")]
        pub enum GenericEnumDeku<T: deku::BitsReader + deku::BitsWriter>
        where
            T: deku::BitsWriter + deku::BitsReader,
        {
            #[deku(id = "1")]
            VariantT(T),
        }
    }

    #[test]
    fn test_unnamed_struct() {
        let test_data: Vec<u8> = [
            0xFF,
            0b1001_0110,
            0xAA,
            0xBB,
            0xCC,
            0xDD,
            0b1001_0110,
            0xCC,
            0xDD,
            0x02,
            0xBE,
            0xEF,
        ]
        .to_vec();

        // Read
        let ret_read = samples::UnNamedDeku::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            samples::UnNamedDeku(
                0xFF,
                0b0000_0010,
                0b0001_0110,
                0xBBAA,
                0xCCDD,
                samples::NestedDeku {
                    nest_a: 0b00_100101,
                    nest_b: 0b10,
                    inner: samples::DoubleNestedDeku { data: 0xDDCC }
                },
                0x02,
                vec![0xBE, 0xEF],
            ),
            ret_read
        );

        // Write
        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    fn test_named_struct() {
        let test_data: Vec<u8> = [
            0xFF,
            0b1001_0110,
            0xAA,
            0xBB,
            0xCC,
            0xDD,
            0b1001_0110,
            0xCC,
            0xDD,
            0x02,
            0xBE,
            0xEF,
        ]
        .to_vec();

        // Read
        let ret_read = samples::NamedDeku::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            samples::NamedDeku {
                field_a: 0xFF,
                field_b: 0b0000_0010,
                field_c: 0b0001_0110,
                field_d: 0xBBAA,
                field_e: 0xCCDD,
                field_f: samples::NestedDeku {
                    nest_a: 0b00_100101,
                    nest_b: 0b10,
                    inner: samples::DoubleNestedDeku { data: 0xDDCC }
                },
                vec_len: 0x02,
                vec_data: vec![0xBE, 0xEF]
            },
            ret_read
        );

        // Write
        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[rstest(input,expected,
        case(&hex!("01AB"), samples::EnumDeku::VarA(0xAB)),
        case(&hex!("0269"), samples::EnumDeku::VarB(0b0110, 0b1001)),
        case(&hex!("0302AABB"), samples::EnumDeku::VarC{field_a: 0x02, field_b: vec![0xAA, 0xBB]}),
        case(&hex!("0402AABB"), samples::EnumDeku::VarD(0x02, vec![0xAA, 0xBB])),
    )]
    fn test_enum(input: &[u8], expected: samples::EnumDeku) {
        let ret_read = samples::EnumDeku::try_from(input).unwrap();
        assert_eq!(expected, ret_read);

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(input.to_vec(), ret_write);
    }

    #[test]
    fn test_dynamic_vec_count() {
        let test_data: Vec<u8> = [0x02, 0xAA, 0xBB].to_vec();

        // Read
        let mut ret_read = samples::VecCountDeku::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            samples::VecCountDeku {
                count: 0x02,
                vec_data: vec![0xAA, 0xBB]
            },
            ret_read
        );

        // Add an item to the vec
        ret_read.vec_data.push(0xFF);
        ret_read.update().unwrap();

        // Write
        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!([0x03, 0xAA, 0xBB, 0xFF].to_vec(), ret_write);
    }

    #[test]
    #[should_panic(
        expected = "Parse(\"error parsing int: out of range integral type conversion attempted\")"
    )]
    fn test_dynamic_vec_count_error() {
        let mut val = samples::VecCountDeku {
            count: 0x02,
            vec_data: vec![0xAA, 0xBB],
        };

        // `count` is a u8, add u8::MAX ++ items and try to update
        for _ in 0..std::u8::MAX {
            val.vec_data.push(0xFF);
        }
        val.update().unwrap();
    }

    #[test]
    fn test_map() {
        let test_data: Vec<u8> = [0x01, 0x02].to_vec();

        let ret_read = samples::MapDeku::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            samples::MapDeku {
                field_a: "1".to_string(),
                field_b: "2".to_string(),
            },
            ret_read
        );
    }

    #[test]
    fn test_reader_writer() {
        let test_data: Vec<u8> = [0x01].to_vec();

        let ret_read = samples::ReaderWriterDeku::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            samples::ReaderWriterDeku {
                field_a: 0x02 // 0x01 + 1 as specified in the reader function
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    fn test_generic_struct_deku() {
        let test_data: Vec<u8> = [0x01].to_vec();

        let ret_read = samples::GenericStructDeku::<u8>::try_from(test_data.as_ref()).unwrap();
        assert_eq!(samples::GenericStructDeku::<u8> { field_a: 0x01 }, ret_read);

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    fn test_generic_enum_deku() {
        let test_data: Vec<u8> = [0x01, 0x02].to_vec();

        let ret_read = samples::GenericEnumDeku::<u8>::try_from(test_data.as_ref()).unwrap();
        assert_eq!(samples::GenericEnumDeku::<u8>::VariantT(0x02), ret_read);

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }
}
