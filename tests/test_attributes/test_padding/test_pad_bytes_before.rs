use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_pad_bytes_before() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "2")]
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
    assert_eq!(vec![0xAA, 0x00, 0x00, 0xDD], ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"not enough data for padding: expected 16 bits got 0 bits\")")]
fn test_pad_bytes_before_not_enough() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xAA];

    let _ret_read = TestStruct::try_from(data.as_ref()).unwrap();
}

#[test]
#[should_panic(
    expected = r#"InvalidParam("Invalid padding param \"(- 2 * 8)\": cannot convert to usize")"#
)]
fn test_pad_bytes_before_read_err() {
    #[derive(PartialEq, Debug, DekuRead)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "-2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD];

    let _ret_read = TestStruct::try_from(data.as_ref()).unwrap();
}

#[test]
#[should_panic(
    expected = r#"InvalidParam("Invalid padding param \"(- 2 * 8)\": cannot convert to usize")"#
)]
fn test_pad_bytes_before_write_err() {
    #[derive(PartialEq, Debug, DekuWrite)]
    struct TestStruct {
        #[deku(pad_bytes_before = "-2")]
        field_a: u8,
        field_b: u8,
    }

    let data = TestStruct {
        field_a: 0xAA,
        field_b: 0xDD,
    };

    let _ret_write: Vec<u8> = data.try_into().unwrap();
}
