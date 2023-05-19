use deku::prelude::*;
use hexlit::hex;
use std::convert::TryFrom;
use std::convert::TryInto;

#[derive(Clone, Copy, PartialEq, Eq, Debug, DekuWrite, DekuRead)]
#[deku(type = "u8")]
#[non_exhaustive]
#[repr(u8)]
pub enum DekuTest {
    /// A
    #[deku(id = "1")]
    A = 0,
    /// B
    #[deku(id = "2")]
    B = 1,
    /// C
    #[deku(id = "3", default)]
    C = 2,
}

fn main() {
    let input = hex!("0A").to_vec();
    let output = hex!("03").to_vec();

    let ret_read = DekuTest::try_from(input.as_slice()).unwrap();
    assert_eq!(DekuTest::C, ret_read);
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(output.to_vec(), ret_write);
}
