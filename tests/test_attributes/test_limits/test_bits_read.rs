use std::convert::{TryFrom, TryInto};

use deku::prelude::*;
use rstest::rstest;

mod test_slice {
    use super::*;

    #[test]
    fn test_bits_read_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            #[deku(bits_read = "16")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                data: test_data.to_vec()
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[rstest(input_bits,
        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(15),

        case(16),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(17),
    )]
    fn test_bits_read_from_field(input_bits: u8) {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            bits: u8,

            #[deku(bits_read = "bits")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [input_bits, 0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                bits: 16,
                data: test_data[1..].to_vec(),
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }
}

mod test_vec {
    use super::*;

    #[test]
    fn test_bits_read_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            #[deku(endian = "little", bits_read = "16")]
            data: Vec<u16>,
        }

        let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                // We should read 16 bits, not 16 elements,
                // thus resulting in a single u16 element
                data: vec![0xbbaa]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[rstest(input_bits,
        #[should_panic(expected = "Incomplete(NeedSize { bits: 16 })")]
        case(15),

        case(16),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 16 })")]
        case(17),
    )]
    fn test_bits_read_from_field(input_bits: u8) {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            bits: u8,

            #[deku(endian = "little", bits_read = "bits")]
            data: Vec<u16>,
        }

        let test_data: Vec<u8> = [input_bits, 0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                bits: 16,

                // We should read 16 bits, not 16 elements,
                // thus resulting in a single u16 element
                data: vec![0xbbaa]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }
}
