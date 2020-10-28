use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_bits_read_static() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(endian = "little", bits_read = "16")]
        data: Vec<u16>,
    }

    let test_data: Vec<u8> = [0xAA, 0xBB].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            // We should read 16 bits, not 16 elements,
            // thus resulting in a single u16 element
            data: vec![0xBBAA]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xFFEE);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0xAA, 0xBB, 0xEE, 0xFF].to_vec(), ret_write);
}

#[test]
fn test_bits_read_from_field() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        bits: u8,

        #[deku(endian = "little", bits_read = "bits")]
        data: Vec<u16>,
    }

    let test_data: Vec<u8> = [0x10, 0xAA, 0xBB].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            bits: 0x10,

            // We should read 16 bits, not 16 elements,
            // thus resulting in a single u16 element
            data: vec![0xBBAA]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xFFEE);

    // `bits` is still 0x02, this is intended. `update` attribute should be
    // used if `bits` is to be updated
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x10, 0xAA, 0xBB, 0xEE, 0xFF].to_vec(), ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"not enough data: expected 16 bits got 0 bits\")")]
fn test_bits_read_error() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        bits: u8,

        #[deku(endian = "little", bits_read = "bits")]
        data: Vec<u16>,
    }

    let test_data: Vec<u8> = [0x11, 0xAA, 0xBB].to_vec();

    let _ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
}
