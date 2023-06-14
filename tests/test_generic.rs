use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_generic_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct<T>
    where
        T: deku::DekuWrite + for<'a> deku::DekuRead<'a>,
    {
        field_a: T,
    }

    let test_data: Vec<u8> = [0x01].to_vec();

    let ret_read = TestStruct::<u8>::try_from(test_data.as_ref()).unwrap();
    assert_eq!(TestStruct::<u8> { field_a: 0x01 }, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_generic_enum() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    enum TestEnum<T>
    where
        T: deku::DekuWrite + for<'a> deku::DekuRead<'a>,
    {
        #[deku(id = "1")]
        VariantT(T),
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestEnum::<u8>::try_from(test_data.as_ref()).unwrap();
    assert_eq!(TestEnum::<u8>::VariantT(0x02), ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_slice_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct<'a> {
        #[deku(count = "2")]
        field_a: &'a [u8],
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: test_data.as_ref()
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
