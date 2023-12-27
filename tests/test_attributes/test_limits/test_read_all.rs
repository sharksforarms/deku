use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_read_all() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct1 {
        #[deku(read_all)]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

    let ret_read = TestStruct1::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct1 {
            data: test_data.to_vec()
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct2 {
        first: u8,
        #[deku(read_all)]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xff, 0xaa, 0xbb].to_vec();

    let ret_read = TestStruct2::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct2 {
            first: 0xff,
            data: test_data[1..].to_vec()
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
