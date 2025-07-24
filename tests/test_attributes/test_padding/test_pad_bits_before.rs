use core::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_pad_bits_before() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(pad_bits_before = "2", bits = 4)]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_01_1001];

    let ret_read = TestStruct::try_from(data.as_slice()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0b10,
            field_b: 0b1001,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0b10_00_1001], ret_write);
}

#[test]
#[should_panic(expected = "Incomplete(NeedSize { bits: 6 })")]
fn test_pad_bits_before_not_enough() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 4)]
        field_a: u8,
        #[deku(pad_bits_before = "6", bits = 2)]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_01_1001];

    let _ret_read = TestStruct::try_from(data.as_slice()).unwrap();
}

#[test]
#[should_panic(expected = r#"InvalidParam("Invalid padding param, cannot convert to usize: - 1")"#)]
fn test_pad_bits_before_read_err() {
    #[derive(PartialEq, Debug, DekuRead)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(pad_bits_before = "-1", bits = 4)]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_01_1001];

    let _ret_read = TestStruct::try_from(data.as_slice()).unwrap();
}

#[test]
#[should_panic(expected = r#"InvalidParam("Invalid padding param, cannot convert to usize: - 1")"#)]
fn test_pad_bits_before_write_err() {
    #[derive(PartialEq, Debug, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(pad_bits_before = "-1", bits = 4)]
        field_b: u8,
    }

    let data = TestStruct {
        field_a: 0b10,
        field_b: 0b1001,
    };

    let _ret_write: Vec<u8> = data.try_into().unwrap();
}
