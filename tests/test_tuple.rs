use core::convert::{TryFrom, TryInto};

use deku::prelude::*;
use hexlit::hex;
use rstest::*;

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum TestEnum {
    #[deku(id = "1")]
    VarA((u8, u16)),
}

#[rstest(input,expected,
    case(&mut hex!("01ABFFAA"), TestEnum::VarA((0xAB, 0xAAFF))),
)]
fn test_enum(input: &mut [u8], expected: TestEnum) {
    let input = input.to_vec();
    let ret_read = TestEnum::try_from(input.as_slice()).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input, ret_write);
}
