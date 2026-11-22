#![cfg(all(feature = "alloc", feature = "bits"))]
// The bits of each literal are grouped by the fields they fill.
#![allow(clippy::unusual_byte_groupings)]
//! `DekuBitField` for structs and for enums whose variants hold fields: such
//! a type joins the run of a struct that holds it as a `bit_field`, and must
//! read and write exactly what its own reader and writer do.

use deku::prelude::*;

/// A flag, as a field of the structs below.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(id_type = "u8", bits = 1, endian = "big", ctx = "_: deku::ctx::Endian")]
enum Kept {
    #[deku(id = 0)]
    No,
    #[deku(id = 1)]
    Yes,
}

/// Two fields that split a register number.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(endian = "big", ctx = "_: deku::ctx::Endian")]
struct Pair {
    #[deku(bits = 3)]
    low: u8,
    #[deku(bits = 5)]
    high: u8,
}

/// Flags, then a field of plain bits.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(endian = "big", ctx = "_: deku::ctx::Endian")]
struct Frame {
    #[deku(bit_field)]
    save: Kept,
    #[deku(bit_field)]
    ra: Kept,
    #[deku(bits = 2, pad_bits_after = "1")]
    statics: u8,
    #[deku(bits = 3)]
    size: u8,
}

/// An id of 3 bits, then 8 bits whatever the variant. Ids `0b101` to
/// `0b111` are not assigned, so that some inputs fail.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(id_type = "u8", bits = 3, endian = "big", ctx = "_: deku::ctx::Endian")]
enum Group {
    #[deku(id = 0b000)]
    Immediate(u8),
    #[deku(id = 0b001)]
    Signed {
        #[deku(bits = 1)]
        negative: bool,
        #[deku(bits = 7)]
        magnitude: u8,
    },
    #[deku(id = 0b010)]
    Pair(#[deku(bit_field)] Pair),
    #[deku(id = 0b011)]
    Frame(#[deku(bit_field)] Frame),
    #[deku(id = 0b100)]
    Spare(#[deku(bits = 8)] u8),
}

/// `Group` after 5 bits of padding, as a halfword holds it after its opcode.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Batched {
    #[deku(pad_bits_before = "5", bit_field)]
    group: Group,
}

/// The same halfword, read through `Group`'s own reader.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Plain {
    #[deku(pad_bits_before = "5")]
    group: Group,
}

#[test]
fn widths_are_the_sums_of_the_fields() {
    assert_eq!(<Pair as DekuBitField>::BITS, 8);
    assert_eq!(<Frame as DekuBitField>::BITS, 8);
    assert_eq!(<Group as DekuBitField>::BITS, 11);
}

#[test]
fn a_struct_reads_its_fields_first_field_highest() {
    assert_eq!(
        Pair::from_bit_run(0b101_00011).unwrap(),
        Pair {
            low: 0b101,
            high: 0b00011
        }
    );
    assert_eq!(
        Frame::from_bit_run(0b1_0_11_0_101).unwrap(),
        Frame {
            save: Kept::Yes,
            ra: Kept::No,
            statics: 0b11,
            size: 0b101,
        }
    );
    // Bits above the type's own are not its business.
    assert_eq!(
        Pair::from_bit_run(0xff00 | 0b101_00011).unwrap(),
        Pair::from_bit_run(0b101_00011).unwrap()
    );
}

#[test]
fn an_enum_reads_its_id_then_its_variant() {
    assert_eq!(
        Group::from_bit_run(0b010_101_00011).unwrap(),
        Group::Pair(Pair {
            low: 0b101,
            high: 0b00011
        })
    );
    assert_eq!(
        Group::from_bit_run(0b001_1_0000101).unwrap(),
        Group::Signed {
            negative: true,
            magnitude: 5
        }
    );
    assert_eq!(
        Group::from_bit_run(0b000_1111_0000).unwrap(),
        Group::Immediate(0xf0)
    );
    assert_eq!(
        Group::Immediate(0xf0).to_bit_run().unwrap(),
        0b000_1111_0000
    );
}

/// Every halfword reads the same through the run as through the readers of
/// its parts, and writes the same back.
#[test]
fn batched_matches_unbatched_for_every_halfword() {
    for halfword in 0..=u16::MAX {
        let wire = halfword.to_be_bytes();
        let batched = Batched::from_bytes((&wire, 0));
        let plain = Plain::from_bytes((&wire, 0));
        match (batched, plain) {
            (Ok((b_rest, b)), Ok((p_rest, p))) => {
                assert_eq!(b_rest, p_rest, "{halfword:#06x}");
                assert_eq!(b.group, p.group, "{halfword:#06x}");
                assert_eq!(
                    b.to_bytes().unwrap(),
                    p.to_bytes().unwrap(),
                    "{halfword:#06x}"
                );
            }
            (Err(b), Err(p)) => assert_eq!(format!("{b:?}"), format!("{p:?}"), "{halfword:#06x}"),
            (b, p) => panic!("batched and unbatched disagree on {halfword:#06x}: {b:?} vs {p:?}"),
        }
    }
}

/// A value that does not fit fails the run's write as it fails its own.
#[test]
fn write_rejects_what_the_field_write_rejects() {
    let wide = Group::Pair(Pair { low: 9, high: 0 });
    let batched = Batched { group: wide }.to_bytes().unwrap_err();
    let plain = Plain { group: wide }.to_bytes().unwrap_err();
    assert_eq!(format!("{batched:?}"), format!("{plain:?}"));
}

/// A run read off a byte boundary still reads what the readers do.
#[test]
fn a_run_off_a_byte_boundary() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Shifted {
        #[deku(bits = 2)]
        head: u8,
        #[deku(pad_bits_before = "5", bit_field)]
        group: Group,
        #[deku(bits = 6)]
        tail: u8,
    }

    let wire = [0b11_00000_0u8, 0b10_10100_0, 0b11_101010];
    let (_, value) = Shifted::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        value,
        Shifted {
            head: 0b11,
            group: Group::Pair(Pair {
                low: 0b101,
                high: 0b00011
            }),
            tail: 0b101010,
        }
    );
    assert_eq!(value.to_bytes().unwrap(), wire);
}
