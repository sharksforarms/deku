use deku::prelude::*;

#[derive(DekuRead, Debug, PartialEq, Eq)]
pub struct Test {
    #[deku(seek_rewind, seek_from_current = "0")]
    byte_a: u8,
    #[deku(seek_from_current = "1", seek_from_end = "-1")]
    byte_b: u8,
}

fn main() {}
