#![cfg(feature = "bits")]

use core::convert::TryFrom;

use deku::bitvec::Lsb0;
use deku::prelude::*;

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct Test {
    #[deku(bits = "4")]
    pub a: u8,
    #[deku(bits = "4")]
    pub b: u8,
}

#[test]
fn test_to_bits_correct() {
    let test_data: &[u8] = &[0xf1];
    let test = Test::try_from(test_data).unwrap();
    let bits = test.to_bits().unwrap();
    assert_eq!(deku::bitvec::bitvec![1, 1, 1, 1, 0, 0, 0, 1], bits);
}

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct TestOver {
    #[deku(bits = "4")]
    pub a: u8,
    #[deku(bits = "4")]
    pub b: u8,
    #[deku(bits = "1")]
    pub c: u8,
}

#[test]
fn test_to_bits_correct_over() {
    let test_data: &[u8] = &[0xf1, 0x80];
    let test = TestOver::from_bytes((test_data, 0)).unwrap().1;
    let bits = test.to_bits().unwrap();
    assert_eq!(deku::bitvec::bitvec![1, 1, 1, 1, 0, 0, 0, 1, 1], bits);
}

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8", bits = "4")]
enum TestEnum {
    #[deku(id = "0b1010")]
    VarA,
}

#[cfg(feature = "bits")]
#[test]
fn test_to_bits_enum() {
    let test_data: &[u8] = &[0b1010_0000];
    let test = TestEnum::from_bytes((test_data, 0)).unwrap().1;
    let bits = test.to_bits().unwrap();
    assert_eq!(deku::bitvec::bitvec![1, 0, 1, 0], bits);
}
