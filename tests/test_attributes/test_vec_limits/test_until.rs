use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_until_static() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(until = "|v: &u8| *v == 0xBB")]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xAA, 0xBB].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            data: vec![0xAA, 0xBB]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xFF);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0xAA, 0xBB, 0xFF].to_vec(), ret_write);
}

#[test]
fn test_until_from_field() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        until: u8,

        #[deku(until = "|v: &u8| *v == *until")]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xBB, 0xAA, 0xBB].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            until: 0xBB,
            data: vec![0xAA, 0xBB]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xFF);

    // `until` is still 0x02, this is intended. `update` attribute should be
    // used if `until` is to be updated
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0xBB, 0xAA, 0xBB, 0xFF].to_vec(), ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"not enough data: expected 8 bits got 0 bits\")")]
fn test_until_error() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        until: u8,

        #[deku(until = "|v: &u8| *v == *until")]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xCC, 0xAA, 0xBB].to_vec();

    let _ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
}
