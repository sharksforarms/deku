use deku::prelude::*;
use hexlit::hex;
use rstest::rstest;
use std::convert::{TryFrom, TryInto};

#[derive(Default, PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStruct {
    #[deku(aligned)]
    field_a: u8,
    #[deku(aligned)]
    field_b: u8,
}

#[rstest(input, expected,
    case(&hex!("0102"), TestStruct {
        field_a: 0x01,
        field_b: 0x02,
    }),
)]
fn test_aligned_read(input: &[u8], expected: TestStruct) {
    let ret_read = TestStruct::try_from(input).unwrap();
    assert_eq!(expected, ret_read);
}

#[rstest(input, expected,
    case(TestStruct {
        field_a: 0x01,
        field_b: 0x02,
    }, hex!("0102").to_vec()),
)]
fn test_aligned_write(input: TestStruct, expected: Vec<u8>) {
    let ret_write: Vec<u8> = input.try_into().unwrap();
    assert_eq!(expected, ret_write);
}

#[derive(Default, PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStructNotAligned {
    #[deku(bits = "4")]
    field_a: u8,
    two: TestStructNotAlignedInner,
}

#[derive(Default, PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStructNotAlignedInner {
    #[deku(aligned)]
    field_a: u8,
    #[deku(aligned)]
    field_b: u8,
}

#[rstest(input, expected,
    #[should_panic(expected = r#"called `Result::unwrap()` on an `Err` value: Parse("error parsing from slice: could not convert slice to array")"#)]
    case(&hex!("f01020"), TestStructNotAligned {
        field_a: 0x0f,
        two: TestStructNotAlignedInner {
            field_a: 0x01,
            field_b: 0x02,
        },
    }),
)]
fn test_aligned_failed(input: &[u8], expected: TestStructNotAligned) {
    let ret_read = TestStructNotAligned::try_from(input).unwrap();
    assert_eq!(expected, ret_read);
}
