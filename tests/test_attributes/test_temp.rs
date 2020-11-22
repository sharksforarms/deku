use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_temp_field() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct {
        #[deku(temp)]
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_b: vec![0x02]
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data[1..].to_vec(), ret_write);
}

#[test]
fn test_temp_field_unnamed() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct(#[deku(temp)] u8, #[deku(count = "field_0")] Vec<u8>);

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(TestStruct(vec![0x02]), ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data[1..].to_vec(), ret_write);
}

#[test]
fn test_temp_enum_field() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    #[deku(type = "u8")]
    enum TestEnum {
        #[deku(id = "0xAB")]
        VarA {
            #[deku(temp)]
            field_a: u8,
            #[deku(count = "field_a")]
            field_b: Vec<u8>,
        },
    };

    let test_data: Vec<u8> = [0xAB, 0x01, 0x02].to_vec();

    let ret_read = TestEnum::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestEnum::VarA {
            field_b: vec![0x02]
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0xAB, 0x02], ret_write);
}
