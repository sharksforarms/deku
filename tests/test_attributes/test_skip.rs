use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

/// Skip
#[test]
fn test_skip() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip)]
        field_b: Option<u8>,
        field_c: u8,
    }

    // Skip `field_b`
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: None, // Default::default()
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

/// Skip and default
#[test]
fn test_skip_default() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip, default = "5")]
        field_b: u8,
        field_c: u8,
    }

    // Skip `field_b` and default it's value to 5
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x05,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

/// Conditional skipping
#[test]
fn test_skip_cond() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip, cond = "*field_a == 0x01", default = "5")]
        field_b: u8,
    }

    // if `cond` is true, skip and default `field_b` to 5
    let test_data: Vec<u8> = [0x01].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x05, // default
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);

    // if `cond` is false, read `field_b` from input
    let test_data: Vec<u8> = [0x02, 0x03].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x02,
            field_b: 0x03,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
