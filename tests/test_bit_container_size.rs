#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::prelude::*;
use rstest::*;

#[derive(Debug, Default, PartialEq, DekuWrite, DekuRead)]
#[deku(endian = "big")]
struct Test {
    #[deku(bits = 4)]
    field_u8_be: u8,
    #[deku(bits = 4)]
    field_be: u16,
    #[deku(endian = "little", bits = 12)]
    field_le: u32,
    #[deku(endian = "big", bits = 9)]
    field_u32_be: u32,
}

#[rstest(input,
    #[should_panic(
        expected = "bit size of input is larger than bit requested size"
    )]
    case::field_u8_be( Test { field_u8_be: 0b11111, ..Test::default()}),
    #[should_panic(
        expected = "bit size of input is larger than bit requested size"
    )]
    case::field_be( Test { field_be: 0b11111, ..Test::default()}),
    #[should_panic(
        expected = "bit size of input is larger than requested size"
    )]
    case::field_le( Test { field_le: 0b1111111111111, ..Test::default()}),
    #[should_panic(
        expected = "bit size of input is larger than bit requested size"
    )]
    case::field_u32_be( Test { field_u32_be: 0b1111111111111, ..Test::default()}),
)]
fn test_bit_container_to_big(input: Test) {
    input.to_bytes().unwrap();
}
