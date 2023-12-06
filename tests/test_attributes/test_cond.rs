use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_cond_deku() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(cond = "*field_a == 0x01")]
        field_b: Option<u8>,
    }

    // `cond` is true
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: Some(0x02)
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);

    // `cond` is false
    let test_data: Vec<u8> = [0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x02,
            field_b: None, // Default::default()
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
