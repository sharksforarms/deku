use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_bytes_read_static() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(endian = "little", bytes_read = "2")]
        data: Vec<u16>,
    }

    let test_data: Vec<u8> = [0xAA, 0xBB].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            // We should read two bytes, not two elements,
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
fn test_bytes_read_from_field() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        bytes: u8,

        #[deku(endian = "little", bytes_read = "bytes")]
        data: Vec<u16>,
    }

    let test_data: Vec<u8> = [0x02, 0xAA, 0xBB].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            bytes: 0x02,

            // We should read two bytes, not two elements,
            // thus resulting in a single u16 element
            data: vec![0xBBAA]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xFFEE);

    // `bytes` is still 0x02, this is intended. `update` attribute should be
    // used if `bytes` is to be updated
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x02, 0xAA, 0xBB, 0xEE, 0xFF].to_vec(), ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"not enough data: expected 16 bits got 0 bits\")")]
fn test_bytes_read_error() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        bytes: u8,

        #[deku(endian = "little", bytes_read = "bytes")]
        data: Vec<u16>,
    }

    let test_data: Vec<u8> = [0x03, 0xAA, 0xBB].to_vec();

    let _ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
}
