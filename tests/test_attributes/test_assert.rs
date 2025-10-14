#[cfg(feature = "descriptive-errors")]
use core::convert::{TryFrom, TryInto};

use deku::prelude::*;
#[cfg(feature = "descriptive-errors")]
use hexlit::hex;
#[cfg(feature = "descriptive-errors")]
use rstest::rstest;

#[derive(Default, PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStruct {
    field_a: u8,
    #[deku(assert = "*field_a + *field_b >= 3")]
    field_b: u8,
}

#[cfg(feature = "descriptive-errors")]
#[rstest(input, expected,
    case(&hex!("0102"), TestStruct {
        field_a: 0x01,
        field_b: 0x02,
    }),

    #[should_panic(expected = r#"Assertion("Field failed assertion: TestStruct.field_b: * field_a + * field_b >= 3")"#)]
    case(&hex!("0101"), TestStruct::default())
)]
fn test_assert_read(input: &[u8], expected: TestStruct) {
    let ret_read = TestStruct::try_from(input).unwrap();
    assert_eq!(expected, ret_read);
}

#[cfg(feature = "descriptive-errors")]
#[rstest(input, expected,
    case(TestStruct {
        field_a: 0x01,
        field_b: 0x02,
    }, hex!("0102").to_vec()),

    #[should_panic(expected = r#"Assertion("Field failed assertion: TestStruct.field_b: * field_a + * field_b >= 3")"#)]
    case(TestStruct {
        field_a: 0x01,
        field_b: 0x01,
    }, hex!("").to_vec()),
)]
fn test_assert_write(input: TestStruct, expected: Vec<u8>) {
    let ret_write: Vec<u8> = input.try_into().unwrap();
    assert_eq!(expected, ret_write);
}
