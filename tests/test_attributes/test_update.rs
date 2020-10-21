use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

/// Update field value
#[test]
fn test_update() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "5")]
        field_a: u8,
    }

    // Update `field_a` to 5
    let test_data: Vec<u8> = [0x01].to_vec();

    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(TestStruct { field_a: 0x01 }, ret_read);

    // `field_a` field should now be increased
    ret_read.update().unwrap();
    assert_eq!(0x05, ret_read.field_a);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x05].to_vec(), ret_write);
}

/// Update from field on `self`
#[test]
fn test_update_from_field() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "self.data.len()")]
        count: u8,
        #[deku(count = "count")]
        data: Vec<u8>,
    }

    // Update the value of `count` to the length of `data`
    let test_data: Vec<u8> = [0x02, 0xAA, 0xBB].to_vec();

    // Read
    let mut ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            count: 0x02,
            data: vec![0xAA, 0xBB]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xFF);

    // `count` field should now be increased
    ret_read.update().unwrap();
    assert_eq!(3, ret_read.count);

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x03, 0xAA, 0xBB, 0xFF].to_vec(), ret_write);
}

/// Update error
#[test]
#[should_panic(
    expected = "Parse(\"error parsing int: out of range integral type conversion attempted\")"
)]
fn test_update_error() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "256")]
        count: u8,
    }

    let mut val = TestStruct { count: 0x01 };

    val.update().unwrap();
}
