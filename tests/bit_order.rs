use assert_hex::assert_eq_hex;
use deku::ctx::{BitSize, Order};
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

    let bytes = control_frame.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
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
    let data: &[u8] = &[0b0111_1001, 0b111_11100];
    let g = ReadGreater::try_from(data).unwrap();

    let bytes = g.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
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

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
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

    let bytes = header.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct Enums {
    right: Choice,
    left: Choice,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
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
            left: Choice::Full
        }
    );

    let bytes = control_frame.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct MoreFirst {
    #[deku(bits = "13")]
    offset: u16,
    #[deku(bits = "3")]
    t: u8,
}

#[test]
fn test_bit_order_more_first() {
    let data = vec![0x40, 0x40];
    let more_first = MoreFirst::try_from(data.as_ref()).unwrap();
    assert_eq!(more_first, MoreFirst { offset: 0x40, t: 2 });

    let bytes = more_first.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct LsbField {
    #[deku(bit_order = "lsb", bits = "13")]
    offset: u16,
    #[deku(bit_order = "lsb", bits = "3")]
    t: u8,
}

#[test]
fn test_bit_order_lsb_field() {
    let data = vec![0x40, 0x40];
    let more_first = LsbField::try_from(data.as_ref()).unwrap();
    assert_eq!(more_first, LsbField { offset: 0x40, t: 2 });

    let bytes = more_first.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[test]
fn test_bit_order_custom_reader_writer() {
    fn reader_lsb<R: std::io::Read>(reader: &mut Reader<R>) -> Result<(u16, u8), DekuError> {
        let first = u16::from_reader_with_ctx(reader, (BitSize(13), Order::Lsb0))?;
        let second = u8::from_reader_with_ctx(reader, BitSize(3))?;

        Ok((first, second))
    }

    fn reader_msb<R: std::io::Read>(reader: &mut Reader<R>) -> Result<(u16, u8), DekuError> {
        let first = u16::from_reader_with_ctx(reader, (BitSize(13), Order::Msb0))?;
        let second = u8::from_reader_with_ctx(reader, BitSize(3))?;

        Ok((first, second))
    }

    fn writer_lsb<W: std::io::Write>(
        val_msb: (u16, u8),
        writer: &mut Writer<W>,
    ) -> Result<(), DekuError> {
        val_msb.0.to_writer(writer, (BitSize(13), Order::Lsb0))?;
        val_msb.1.to_writer(writer, (BitSize(3), Order::Msb0))?;

        Ok(())
    }

    fn writer_msb<W: std::io::Write>(
        val_msb: (u16, u8),
        writer: &mut Writer<W>,
    ) -> Result<(), DekuError> {
        val_msb.0.to_writer(writer, (BitSize(13), Order::Msb0))?;
        val_msb.1.to_writer(writer, (BitSize(3), Order::Msb0))?;

        Ok(())
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    pub struct Custom {
        #[deku(reader = "reader_lsb(deku::reader)")]
        #[deku(writer = "writer_lsb(*val_lsb, deku::writer)")]
        val_lsb: (u16, u8),
        #[deku(reader = "reader_msb(deku::reader)")]
        #[deku(writer = "writer_msb(*val_msb, deku::writer)")]
        val_msb: (u16, u8),
    }

    //              |lsb                    |msb
    //              | f          |sss|rest f|  f                 |sss|
    let data = vec![0b0000_0000, 0b0011_1111, 0b0100_0000, 0b0011_0000];
    let more_first = Custom::try_from(data.as_ref()).unwrap();
    assert_eq!(
        more_first,
        Custom {
            val_lsb: (0b1_1111_0000_0000, 1),
            val_msb: (0b0_0110_0100_0000, 0)
        }
    );

    let bytes = more_first.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "big", bit_order = "lsb")]
pub struct MoreFirstBe {
    #[deku(bits = "13")]
    offset: u16,
    #[deku(bits = "3")]
    t: u8,
}

#[test]
fn test_bit_order_more_first_be() {
    let data = vec![0x40, 0x40];
    let more_first = MoreFirstBe::try_from(data.as_ref()).unwrap();
    assert_eq!(
        more_first,
        MoreFirstBe {
            offset: 0x4000,
            t: 2
        }
    );

    let bytes = more_first.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "little", bit_order = "lsb")]
pub struct BitOrderLittle {
    #[deku(bits = 4)]
    value_a: u16,

    #[deku(bits = 11)]
    value_b: u16,

    #[deku(bits = 13)]
    value_c: u16,

    #[deku(bits = 10)]
    value_d: u16,

    #[deku(bits = 8)]
    value_e: u16,

    #[deku(bits = 9)]
    value_f: u16,

    #[deku(bits = 9)]
    value_g: u16,

    #[deku(bits = 8)]
    value_h: u16,

    #[deku(bits = 7)]
    value_i: u16,

    #[deku(bits = 9)]
    value_j: u16,
}

#[test]
fn test_bit_order_little() {
    let data = vec![
        0x8B, 0xF3, 0xDC, 0x7B, 0x94, 0x38, 0x98, 0x42, 0x78, 0xB8, 0x5E,
    ];
    let bit_order_little = BitOrderLittle::try_from(data.as_ref()).unwrap();
    assert_eq!(
        bit_order_little,
        BitOrderLittle {
            value_a: 11,
            value_b: 1848,
            value_c: 6073,
            value_d: 327,
            value_e: 226,
            value_f: 96,
            value_g: 133,
            value_h: 120,
            value_i: 56,
            value_j: 189,
        }
    );

    let bytes = bit_order_little.to_bytes().unwrap();
    assert_eq_hex!(bytes, data);
}
