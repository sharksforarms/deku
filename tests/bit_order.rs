use deku::ctx::Order;
use deku::prelude::*;
use hexlit::hex;
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

#[test]
fn test_bit_order_frame() {
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

#[derive(Debug, DekuRead, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct ReadGreater {
    #[deku(bits = "1")]
    one: u8,
    #[deku(bits = "2")]
    two: u8,
    #[deku(bits = "4")]
    three: u8,
    #[deku(bits = "3")]
    four: u8,
    #[deku(bits = "6")]
    five: u8,
}

#[test]
fn test_bit_order_read_greater() {
    let data: &[u8] = &[0b01111001, 0b11111100];
    let g = ReadGreater::try_from(data).unwrap();
}

#[derive(Debug, DekuRead, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct SquashfsV3 {
    #[deku(bits = "4")]
    inode_type: u32,
    #[deku(bits = "12")]
    mode: u32,
    #[deku(bits = "8")]
    uid: u32,
    #[deku(bits = "8")]
    guid: u32,
    mtime: u32,
    inode_number: u32,
}

#[test]
fn test_bit_order_squashfs() {
    let data: &[u8] = &[
        0x31, 0x12, 0x04, 0x05, 0x06, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
    ];
    let header = SquashfsV3::try_from(data).unwrap();
    assert_eq!(
        SquashfsV3 {
            inode_type: 0x01,
            mode: 0x123,
            uid: 0x4,
            guid: 0x5,
            mtime: 0x6,
            inode_number: 0x7
        },
        header,
    );
}

#[derive(Debug, DekuRead, PartialEq)]
pub struct Surrounded {
    one: u8,
    header: SquashfsV3,
    two: u8,
    #[deku(bit_order = "lsb", bits = "4")]
    three: u8,
    #[deku(bits = "4")]
    four: u8,
    #[deku(bits = "4")]
    five: u8,
    #[deku(bit_order = "lsb", bits = "4")]
    six: u8,
}

#[test]
fn test_bit_order_surrounded() {
    let data: &[u8] = &[
        0xff, 0x31, 0x12, 0x04, 0x05, 0x06, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0xff, 0x0f,
        0x0f,
    ];
    let header = Surrounded::try_from(data).unwrap();
    assert_eq!(
        Surrounded {
            one: 0xff,
            header: SquashfsV3 {
                inode_type: 0x01,
                mode: 0x123,
                uid: 0x4,
                guid: 0x5,
                mtime: 0x6,
                inode_number: 0x7
            },
            two: 0xff,
            three: 0xf,
            four: 0x0,
            five: 0x0,
            six: 0xf,
        },
        header
    );
}

#[derive(Debug, DekuRead, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct Enums {
    right: Choice,
    left: Choice,
}

#[derive(Debug, DekuRead, PartialEq)]
#[deku(
    bits = "4",
    type = "u8",
    bit_order = "bit_order",
    ctx = "bit_order: deku::ctx::Order"
)]
pub enum Choice {
    Empty = 0x0,
    Full = 0xf,
}

#[test]
fn test_bit_order_enums() {
    let data = vec![0xf0];
    let control_frame = Enums::try_from(data.as_ref()).unwrap();
    assert_eq!(
        control_frame,
        Enums {
            right: Choice::Empty,
            left: Choice::Full,
        }
    );
}

#[derive(Debug, DekuRead, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct MoreFirst {
    #[deku(bits = "13")]
    offset: u16,
    #[deku(bits = "3")]
    t: u8,
}

#[test]
fn test_bit_order_more_first() {
    // |offset                 |t  t  t |offset       |
    // [0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]
    // 0100_0000b
    let data = vec![0x40, 0x40];
    let more_first = MoreFirst::try_from(data.as_ref()).unwrap();
    //                           bits: 13            3
    assert_eq!(more_first, MoreFirst { offset: 0x40, t: 2 });
}
