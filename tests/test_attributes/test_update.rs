use core::convert::{TryFrom, TryInto};

use deku::prelude::*;

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

    let mut ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
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
    let test_data: Vec<u8> = [0x02, 0xaa, 0xbb].to_vec();

    // Read
    let mut ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            count: 0x02,
            data: vec![0xaa, 0xbb]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.data.push(0xff);

    // `count` field should now be increased
    ret_read.update().unwrap();
    assert_eq!(3, ret_read.count);

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x03, 0xaa, 0xbb, 0xff].to_vec(), ret_write);
}

/// Update error
#[test]
#[cfg(feature = "descriptive-errors")]
#[should_panic(expected = "Parse(\"error parsing int: ")]
fn test_update_error() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(update = "256")]
        count: u8,
    }

    let mut val = TestStruct { count: 0x01 };

    val.update().unwrap();
}

/// `update` on a field inside a batched bit-field run still rewrites it, because
/// it feeds only the `DekuUpdate` impl.
#[test]
#[cfg(feature = "bits")]
fn test_update_in_bit_run() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct TestStruct {
        #[deku(bits = 4)]
        field_a: u8,
        #[deku(bits = 4, update = "9")]
        field_b: u8,
        field_c: u8,
    }

    let mut ret_read = TestStruct::try_from([0x12u8, 0x34].as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x1,
            field_b: 0x2,
            field_c: 0x34
        },
        ret_read
    );

    ret_read.update().unwrap();
    assert_eq!(0x9, ret_read.field_b);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x19, 0x34], ret_write);
}
