#[cfg(test)]
mod tests {
    use deku::prelude::*;
    use std::convert::TryFrom;

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
        pub struct VecCountDeku {
            pub count: u8,
            #[deku(len = "count")]
            pub vec_data: Vec<u8>,
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
        let ret_write: Vec<u8> = ret_read.into();
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
        let ret_write: Vec<u8> = ret_read.into();
        assert_eq!(test_data, ret_write);
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
        ret_read.update();

        // Write
        let ret_write: Vec<u8> = ret_read.into();
        assert_eq!([0x03, 0xAA, 0xBB, 0xFF].to_vec(), ret_write);
    }
}
