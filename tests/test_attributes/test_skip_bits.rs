use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_skip_bits() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(skip_bits = "2", bits = 4)]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_01_1001];

    let ret_read = TestStruct::try_from(data.as_ref()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0b10,
            field_b: 0b1001,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0b10_1001_00], ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"not enough data: expected 4 bits got 0 bits\")")]
fn test_skip_bits_not_enough() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(skip_bits = "6", bits = 4)]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_01_1001];

    let _ret_read = TestStruct::try_from(data.as_ref()).unwrap();
}

#[test]
fn test_skip_bits_and_bytes() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(skip_bits = "6", skip_bytes = "1")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_000000, 0xAA, 0xBB];

    let ret_read = TestStruct::try_from(data.as_ref()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0b10,
            field_b: 0xBB,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0b10_101110, 0b11_000000], ret_write);
}
