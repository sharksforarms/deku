use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_skip_bytes() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip_bytes = "2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD];

    let ret_read = TestStruct::try_from(data.as_ref()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0xAA,
            field_b: 0xDD,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0xAA, 0xDD], ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"not enough data: expected 16 bits got 0 bits\")")]
fn test_skip_bytes_not_enough() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip_bytes = "2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xAA];

    let _ret_read = TestStruct::try_from(data.as_ref()).unwrap();
}
