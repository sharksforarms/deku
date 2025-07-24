#![cfg(feature = "alloc")]

extern crate alloc;
use alloc::vec::Vec;

use core::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_generic_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct<T>
    where
        T: deku::DekuWriter + for<'a> deku::DekuReader<'a>,
    {
        field_a: T,
    }

    let test_data: Vec<u8> = [0x01].to_vec();

    let ret_read = TestStruct::<u8>::try_from(test_data.as_slice()).unwrap();
    assert_eq!(TestStruct::<u8> { field_a: 0x01 }, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_generic_enum() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(id_type = "u8")]
    enum TestEnum<T>
    where
        T: deku::DekuWriter + for<'a> deku::DekuReader<'a>,
    {
        #[deku(id = "1")]
        VariantT(T),
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestEnum::<u8>::try_from(test_data.as_slice()).unwrap();
    assert_eq!(TestEnum::<u8>::VariantT(0x02), ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
