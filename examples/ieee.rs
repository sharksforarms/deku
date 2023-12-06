use deku::ctx::Order;
use deku::prelude::*;

use std::convert::TryFrom;

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(type = "u8", bits = "2")]
#[deku(bit_order = "ctx_lsb", ctx = "ctx_lsb: Order")]
pub enum FrameType {
    #[deku(id = "0")]
    Management,
    #[deku(id = "1")]
    Control,
    #[deku(id = "2")]
    Data,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(bit_order = "ctx_lsb", ctx = "ctx_lsb: Order")]
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

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct FrameControl {
    #[deku(bits = 4)]
    pub sub_type: u8,
    #[deku(bits = 2)]
    pub protocol_version: u8,
    pub frame_type: FrameType,

    pub flags: Flags,
}

fn main() {
    let data = vec![0x88u8, 0x41];
    let control_frame = FrameControl::try_from(data.as_ref()).unwrap();
    assert_eq!(
        control_frame,
        FrameControl {
            protocol_version: 0,
            frame_type: FrameType::Data,
            sub_type: 8,

            flags: Flags {
                to_ds: 1,
                from_ds: 0,
                more_fragments: 0,
                retry: 0,
                power_management: 0,
                more_data: 0,
                protected_frame: 1,
                order: 0,
            }
        }
    );
}
