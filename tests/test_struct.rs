#![allow(clippy::unusual_byte_groupings)]

use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

mod test_common;

/// General smoke tests for structs
/// TODO: These should be divided into smaller tests

// Common struct to test nesting
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct DoubleNestedDeku {
    pub data: u16,
}

// Common struct to test nesting
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct NestedDeku {
    #[deku(bits = "6")]
    pub nest_a: u8,
    #[deku(bits = "2")]
    pub nest_b: u8,

    pub inner: DoubleNestedDeku,
}

#[test]
#[should_panic(expected = r#"Parse("Too much data")"#)]
fn test_read_too_much_data() {
    #[derive(DekuRead)]
    pub struct TestStruct {
        #[deku(bits = "6")]
        pub field_a: u8,
    }

    let test_data = [0u8; 100].as_ref();
    TestStruct::try_from(test_data).unwrap();
}

#[test]
fn test_unnamed_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct TestUnamedStruct(
        pub u8,
        #[deku(bits = "2")] pub u8,
        #[deku(bits = "6")] pub u8,
        #[deku(bytes = "2")] pub u16,
        #[deku(endian = "big")] pub u16,
        pub NestedDeku,
        #[deku(update = "self.7.len()")] pub u8,
        #[deku(count = "field_6")] pub Vec<u8>,
    );

    let test_data: Vec<u8> = [
        0xff,
        0b1001_0110,
        0xaa,
        0xbb,
        0xcc,
        0xdd,
        0b1001_0110,
        0xcc,
        0xdd,
        0x02,
        0xbe,
        0xef,
    ]
    .to_vec();

    // Read
    let ret_read = TestUnamedStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestUnamedStruct(
            0xff,
            0b0000_0010,
            0b0001_0110,
            native_endian!(0xbbaau16),
            0xccddu16,
            NestedDeku {
                nest_a: 0b00_100101,
                nest_b: 0b10,
                inner: DoubleNestedDeku {
                    data: native_endian!(0xddccu16)
                }
            },
            0x02,
            vec![0xbe, 0xef],
        ),
        ret_read
    );

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_named_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct TestStruct {
        pub field_a: u8,
        #[deku(bits = "2")]
        pub field_b: u8,
        #[deku(bits = "6")]
        pub field_c: u8,
        #[deku(bytes = "2")]
        pub field_d: u16,
        #[deku(update = "1+self.field_d")]
        #[deku(endian = "big")]
        pub field_e: u16,
        pub field_f: NestedDeku,
        pub vec_len: u8,
        #[deku(count = "vec_len")]
        pub vec_data: Vec<u8>,
    }

    let test_data: Vec<u8> = [
        0xff,
        0b1001_0110,
        0xaa,
        0xbb,
        0xcc,
        0xdd,
        0b1001_0110,
        0xcc,
        0xdd,
        0x02,
        0xbe,
        0xef,
    ]
    .to_vec();

    // Read
    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0xff,
            field_b: 0b0000_0010,
            field_c: 0b0001_0110,
            field_d: native_endian!(0xbbaau16),
            field_e: 0xccddu16,
            field_f: NestedDeku {
                nest_a: 0b00_100101,
                nest_b: 0b10,
                inner: DoubleNestedDeku {
                    data: native_endian!(0xddccu16)
                }
            },
            vec_len: 0x02,
            vec_data: vec![0xbe, 0xef]
        },
        ret_read
    );

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_raw_identifiers_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct TestStruct {
        pub r#type: u8,
    }

    let test_data: Vec<u8> = [0xff].to_vec();

    // Read
    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(TestStruct { r#type: 0xff }, ret_read);

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
