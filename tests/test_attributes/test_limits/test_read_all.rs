use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_read_all() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct1 {
        #[deku(read_all)]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xaa, 0xbb].to_vec();

    let ret_read = TestStruct1::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct1 {
            data: test_data.to_vec()
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct2 {
        first: u8,
        #[deku(read_all)]
        data: Vec<u8>,
    }

    let test_data: Vec<u8> = [0xff, 0xaa, 0xbb].to_vec();

    let ret_read = TestStruct2::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct2 {
            first: 0xff,
            data: test_data[1..].to_vec()
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[cfg(feature = "bits")]
#[test]
fn test_485_read_all_at_end() {
    #[derive(Clone, Debug, DekuWrite, DekuRead, PartialEq)]
    pub struct Foo {
        #[deku(bits = 40)]
        pub weird_sized_thing: u64,
        pub normal_sized_thing: u8,
        pub other_thing: u16,
        #[deku(read_all)]
        pub bulk_things: Vec<u8>,
    }

    #[rustfmt::skip]
    const INPUT_DATA_MESSAGE: [u8; 11] = [
        0x01, 0x01, 0x01, 0x01, 0x01,
        0x0,
        0x02, 0x02,
        0x00, 0x00, 0x00,
    ];

    let foo = Foo::try_from(INPUT_DATA_MESSAGE.as_ref()).unwrap();
    assert_eq!(
        foo,
        Foo {
            weird_sized_thing: 0x101010101,
            normal_sized_thing: 0x0,
            other_thing: 0x0202,
            bulk_things: vec![0x00, 0x00, 0x00]
        }
    );
}
