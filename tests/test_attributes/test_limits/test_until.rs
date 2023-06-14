use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

mod test_slice {
    use super::*;

    #[test]
    fn test_until_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct<'a> {
            #[deku(until = "|v: &u8| *v == 0xBB")]
            data: &'a [u8],
        }

        let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

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

    #[test]
    fn test_until_from_field() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct<'a> {
            until: u8,

            #[deku(until = "|v: &u8| *v == *until")]
            data: &'a [u8],
        }

        let test_data: Vec<u8> = [0xbb, 0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            TestStruct {
                until: 0xbb,
                data: &test_data[1..]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
    fn test_until_error() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct<'a> {
            until: u8,

            #[deku(until = "|v: &u8| *v == *until")]
            data: &'a [u8],
        }

        let test_data: Vec<u8> = [0xcc, 0xaa, 0xbb].to_vec();

        let _ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    }
}

mod test_vec {
    use super::*;

    #[test]
    fn test_until_static() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            #[deku(until = "|v: &u8| *v == 0xBB")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
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
    fn test_until_from_field() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            until: u8,

            #[deku(until = "|v: &u8| *v == *until")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0xbb, 0xaa, 0xbb].to_vec();

        let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
        assert_eq!(
            TestStruct {
                until: 0xbb,
                data: vec![0xaa, 0xbb]
            },
            ret_read
        );

        let ret_write: Vec<u8> = ret_read.try_into().unwrap();
        assert_eq!(test_data, ret_write);
    }

    #[test]
    #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
    fn test_until_error() {
        #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
        struct TestStruct {
            until: u8,

            #[deku(until = "|v: &u8| *v == *until")]
            data: Vec<u8>,
        }

        let test_data: Vec<u8> = [0xcc, 0xaa, 0xbb].to_vec();

        let _ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    }
}
