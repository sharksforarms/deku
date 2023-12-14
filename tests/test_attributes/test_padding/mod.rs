use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

mod test_pad_bits_after;
mod test_pad_bits_before;
mod test_pad_bytes_after;
mod test_pad_bytes_before;

#[test]
#[allow(clippy::identity_op)]
fn test_pad_bits_before_and_pad_bytes_before() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2)]
        field_a: u8,
        #[deku(pad_bits_before = "5 + 1", pad_bytes_before = "0 + 1")]
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_000000, 0xaa, 0xbb];

    let ret_read = TestStruct::try_from(data.as_slice()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0b10,
            field_b: 0xbb,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0b10_000000, 0x00, 0xbb], ret_write);
}

#[test]
fn test_pad_bits_after_and_pad_bytes_after() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(bits = 2, pad_bits_after = "6", pad_bytes_after = "1")]
        field_a: u8,
        field_b: u8,
    }

    let data: Vec<u8> = vec![0b10_000000, 0xaa, 0xbb];

    let ret_read = TestStruct::try_from(data.as_slice()).unwrap();

    assert_eq!(
        TestStruct {
            field_a: 0b10,
            field_b: 0xbb,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0b10_000000, 0x00, 0xbb], ret_write);
}
