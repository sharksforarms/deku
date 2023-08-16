use std::convert::{TryFrom, TryInto};

use deku::prelude::*;
use hexlit::hex;
use rstest::rstest;

#[rstest(input,
    case(&hex!("64656b75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64656bde")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("6465ad75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64be6b75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("ef656b75")),

    #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
    case(&hex!("64656b")),
)]
fn test_magic_struct(input: &[u8]) {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(magic = b"deku")]
    struct TestStruct {}
    let mut input = input.to_vec();
    let ret_read = TestStruct::try_from(input.as_mut_slice()).unwrap();

    assert_eq!(TestStruct {}, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, input)
}

#[rstest(input,
    case(&hex!("64656b7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64656bde00")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("6465ad7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64be6b7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("ef656b7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64656b00")),

    #[should_panic(expected = "Incomplete(NeedSize { bits: 8 })")]
    case(&hex!("64656b")),
)]
fn test_magic_enum(input: &[u8]) {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(magic = b"deku", type = "u8")]
    enum TestEnum {
        #[deku(id = "0")]
        Variant,
    }
    let mut input = input.to_vec();

    let ret_read = TestEnum::try_from(input.as_mut_slice()).unwrap();

    assert_eq!(TestEnum::Variant, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, input)
}
