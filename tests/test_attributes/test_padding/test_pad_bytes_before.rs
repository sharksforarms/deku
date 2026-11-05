use core::convert::{TryFrom, TryInto};

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

#[cfg(feature = "descriptive-errors")]
fn read_pad_bytes_before_not_enough() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(pad_bytes_before = "2")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0xaa];

    let _ret_read = TestStruct::try_from(data.as_slice()).unwrap();
}

// With bits enabled, padding is skipped by seeking, so the next field reports EOF.
#[test]
#[cfg(all(feature = "descriptive-errors", feature = "bits"))]
#[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
fn test_pad_bytes_before_not_enough_with_bits() {
    read_pad_bytes_before_not_enough();
}

// Without bits, the padding read reports the full two-byte request on EOF.
#[test]
#[cfg(all(feature = "descriptive-errors", not(feature = "bits")))]
#[should_panic(expected = "Incomplete(NeedSize { bits: 16 })")]
fn test_pad_bytes_before_not_enough_without_bits() {
    read_pad_bytes_before_not_enough();
}

// TODO: add cfg test with updated msg for not(bits)
#[cfg(all(feature = "bits", feature = "descriptive-errors"))]
#[test]
#[should_panic(
    expected = r#"InvalidParam("Invalid padding param, cannot convert to usize: ((- 2) * 8)")"#
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
#[test]
#[cfg(all(feature = "bits", feature = "descriptive-errors"))]
#[should_panic(
    expected = r#"InvalidParam("Invalid padding param, cannot convert to usize: ((- 2) * 8)")"#
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
