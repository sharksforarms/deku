use deku::prelude::*;
use hexlit::hex;
use rstest::*;

use std::convert::TryFrom;

#[derive(DekuRead, Debug, PartialEq, Eq)]
pub struct Test {
    // how many following bytes to skip
    skip_u8: u8,
    #[deku(seek_from_current = "*skip_u8")]
    byte: u8,
}

#[rstest(input, expected,
    case(&hex!("010020"), Test{ skip_u8: 1, byte: 0x20 }),
)]
fn test_seek_from_current(input: &[u8], expected: Test) {
    let input = input.to_vec();
    // TODO: fix, "too much data"
    //let ret_read = Test::try_from(input.as_slice()).unwrap();

    let mut cursor = std::io::Cursor::new(input);
    let (_, ret_read) = Test::from_reader((&mut cursor, 0)).unwrap();

    assert_eq!(ret_read, expected);
}
