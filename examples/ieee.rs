use deku::prelude::*;
use hexlit::hex;
use std::convert::TryFrom;

#[derive(Debug, DekuRead, PartialEq)]
#[deku(bit_order = "lsb")]
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

fn main() {
    env_logger::init();
    let data: &[u8] = &[0b10011100];
    let frame = Flags::try_from(data).unwrap();
    assert_eq!(
        Flags {
            to_ds: 0,
            from_ds: 0,
            more_fragments: 1,
            retry: 1,
            power_management: 1,
            more_data: 0,
            protected_frame: 0,
            order: 1,
        },
        frame
    );

    let data: &[u8] = &[0b01111001, 0b11111100];
    let g = ReadGreater::try_from(data).unwrap();

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
