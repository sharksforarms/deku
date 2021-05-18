use deku::prelude::*;
use hexlit::hex;
use rstest::rstest;
use std::convert::{TryFrom, TryInto};

#[derive(Default, PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStruct {
    field_a: u8,
    #[deku(assert_eq = "*field_a")]
    field_b: u8,
}

#[rstest(input, expected,
    case(&hex!("0101"), TestStruct {
        field_a: 0x01,
        field_b: 0x01,
    }),

    #[should_panic(expected = r#"Assertion("field field_b failed assertion: field_b == * field_a")"#)]
    case(&hex!("0102"), TestStruct::default())
)]
fn test_assert_eq_read(input: &[u8], expected: TestStruct) {
    let ret_read = TestStruct::try_from(input).unwrap();
    assert_eq!(expected, ret_read);
}

#[rstest(input, expected,
    case(TestStruct {
        field_a: 0x01,
        field_b: 0x01,
    }, hex!("0101").to_vec()),

    #[should_panic(expected = r#"Assertion("field field_b failed assertion: field_b == * field_a")"#)]
    case(TestStruct {
        field_a: 0x01,
        field_b: 0x02,
    }, hex!("").to_vec()),
)]
fn test_assert_eq_write(input: TestStruct, expected: Vec<u8>) {
    let ret_write: Vec<u8> = input.try_into().unwrap();
    assert_eq!(expected, ret_write);
}
