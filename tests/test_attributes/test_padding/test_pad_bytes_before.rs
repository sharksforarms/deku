use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_pad_bytes_before() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xaa, 0xbb, 0xcc, 0xdd];

    let ret_read = TestStruct::try_from(data.as_slice()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0xaa,
            field_b: 0xdd,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0xaa, 0x00, 0x00, 0xdd], ret_write);
}

#[test]
#[should_panic(expected = "Incomplete(NeedSize { bits: 16 })")]
fn test_pad_bytes_before_not_enough() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xaa];

    let _ret_read = TestStruct::try_from(data.as_slice()).unwrap();
}

// TODO: add cfg test with updated msg for not(bits)
#[cfg(feature = "bits")]
#[test]
#[should_panic(
    expected = r#"InvalidParam("Invalid padding param \"(((- 2) * 8))\": cannot convert to usize")"#
)]
fn test_pad_bytes_before_read_err() {
    #[derive(PartialEq, Debug, DekuRead)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "-2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xaa, 0xbb, 0xcc, 0xdd];

    let _ret_read = TestStruct::try_from(data.as_slice()).unwrap();
}

// TODO: add cfg test with updated msg for not(bits)
#[cfg(feature = "bits")]
#[test]
#[should_panic(
    expected = r#"InvalidParam("Invalid padding param \"(((- 2) * 8))\": cannot convert to usize")"#
)]
fn test_pad_bytes_before_write_err() {
    #[derive(PartialEq, Debug, DekuWrite)]
    struct TestStruct {
        #[deku(pad_bytes_before = "-2")]
        field_a: u8,
        field_b: u8,
    }

    let data = TestStruct {
        field_a: 0xaa,
        field_b: 0xdd,
    };

    let _ret_write: Vec<u8> = data.try_into().unwrap();
}
