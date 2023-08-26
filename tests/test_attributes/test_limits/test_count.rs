use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

mod test_slice {
    use super::*;

    #[test]
    fn test_count_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            #[deku(count = "2")]
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

    #[test]
    fn test_count_from_field() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            count: u8,
            #[deku(count = "count")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0x02, 0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                count: 0x02,
                data: test_data[1..].to_vec(),
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
    fn test_count_error() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            count: u8,
            #[deku(count = "count")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0x03, 0xaa, 0xbb].to_vec();

        let _ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    }
}

mod test_vec {
    use super::*;

    #[test]
    fn test_count_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            #[deku(count = "2")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                data: vec![0xaa, 0xbb]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    fn test_count_from_field() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            count: u8,
            #[deku(count = "count")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0x02, 0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
        assert_eq!(
            TestStruct {
                count: 0x02,
                data: vec![0xaa, 0xbb]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
    fn test_count_error() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            count: u8,
            #[deku(count = "count")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0x03, 0xaa, 0xbb].to_vec();

        let _ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    }
}
