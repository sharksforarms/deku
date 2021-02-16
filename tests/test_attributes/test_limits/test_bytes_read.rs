use deku::prelude::*;
use rstest::rstest;
use std::convert::{TryFrom, TryInto};

mod test_slice {
    use super::*;

    #[test]
    fn test_bytes_read_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct<'a> {
            #[deku(bytes_read = "2")]
            data: &'a [u8],
        }

        let test_data: Vec<u8> = [0xAA, 0xBB].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            TestStruct {
                data: test_data.as_ref()
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[rstest(input_bytes,
        case(2),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
        case(3),
    )]
    fn test_bytes_read_from_field(input_bytes: u8) {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct<'a> {
            bytes: u8,

            #[deku(bytes_read = "bytes")]
            data: &'a [u8],
        }

        let test_data: Vec<u8> = [input_bytes, 0xAA, 0xBB].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            TestStruct {
                bytes: 0x02,
                data: &test_data[1..]
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
    fn test_bytes_read_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            #[deku(endian = "little", bytes_read = "2")]
            data: Vec<u16>,
        }

        let test_data: Vec<u8> = [0xAA, 0xBB].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            TestStruct {
                // We should read two bytes, not two elements,
                // thus resulting in a single u16 element
                data: vec![0xBBAA]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[rstest(input_bytes,
        case(2),

        #[should_panic(expected = "Incomplete(NeedSize { bits: 16 })")]
        case(3),
    )]
    fn test_bytes_read_from_field(input_bytes: u8) {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            bytes: u8,

            #[deku(endian = "little", bytes_read = "bytes")]
            data: Vec<u16>,
        }

        let test_data: Vec<u8> = [input_bytes, 0xAA, 0xBB].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            TestStruct {
                bytes: 0x02,

                // We should read two bytes, not two elements,
                // thus resulting in a single u16 element
                data: vec![0xBBAA]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }
}
