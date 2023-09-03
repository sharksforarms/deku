use deku::prelude::*;
use hexlit::hex;
use std::convert::TryFrom;

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
// #[deku(order = "lsb")]
pub struct Flags {
    #[deku(bits = 1)]
    pub to_ds: u8,
    #[deku(bits = 1)]
    pub from_ds: u8,
    #[deku(bits = 1)]
    pub more_fragments: u8,
    #[deku(bits = 1)]
    pub retry: u8,
    #[deku(bits = 1)]
    pub power_management: u8,
    #[deku(bits = 1)]
    pub more_data: u8,
    #[deku(bits = 1)]
    pub protected_frame: u8,
    #[deku(bits = 1)]
    pub order: u8,
}

fn main() {
    env_logger::init();
    let data: &[u8] = &[0b10011100];
    let frame = Flags::try_from(data).unwrap();
    assert_eq!(Flags {
        to_ds: 0,
        from_ds: 0,
        more_fragments: 1,
        retry: 1,
        power_management: 1,
        more_data: 0,
        protected_frame: 0,
        order: 1,
    }, frame);
    assert_eq!(frame.to_bytes().unwrap(), data);
}
