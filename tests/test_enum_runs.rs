#![cfg(all(feature = "alloc", feature = "bits"))]
// The bits of each literal are grouped by the fields they fill.
#![allow(clippy::unusual_byte_groupings)]
//! An enum whose id and variants are all bits of a run reads its id with
//! the bits after it in one read. It must read what the same enum read one
//! field at a time does.

use deku::prelude::*;

/// A flag of the 32-bit variant.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(id_type = "u8", bits = 1, endian = "big", ctx = "_: deku::ctx::Endian")]
enum Mode {
    #[deku(id = 0)]
    Stay,
    #[deku(id = 1)]
    Exchange,
}

/// A register and an 8-bit constant.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(endian = "big", ctx = "_: deku::ctx::Endian")]
struct Ri {
    #[deku(bits = 3)]
    rx: u8,
    imm: u8,
}

/// Two registers and a 5-bit constant.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(endian = "big", ctx = "_: deku::ctx::Endian")]
struct Rri {
    #[deku(bits = 3)]
    rx: u8,
    #[deku(bits = 3)]
    ry: u8,
    #[deku(bits = 5)]
    imm: u8,
}

/// A jump of a 26-bit target, which takes the halfword after it too.
#[derive(Copy, Clone, Debug, PartialEq, DekuRead, DekuWrite, DekuBitField)]
#[deku(endian = "big", ctx = "_: deku::ctx::Endian")]
struct Jump {
    #[deku(bit_field)]
    mode: Mode,
    #[deku(bits = 10)]
    high: u16,
    low: u16,
}

/// Declares one instruction enum, with `$bits` as the width of its id, so
/// that a literal width and an expression of the same width give the
/// batched and the unbatched read of the same enum.
macro_rules! instruction {
    ($name:ident, $bits:tt) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(id_type = "u8", bits = $bits, endian = "big", ctx = "_: deku::ctx::Endian", ctx_default = "deku::ctx::Endian::Big")]
        enum $name {
            #[deku(id = 0b00000)]
            Addiu(#[deku(bit_field)] Ri),
            #[deku(id = 0b00011)]
            Jump(#[deku(bit_field)] Jump),
            #[deku(id = 0b10011)]
            Load(#[deku(bit_field)] Rri),
            #[deku(id = 0b10100)]
            Load2 {
                #[deku(bit_field)]
                operands: Rri,
            },
            #[deku(id = 0b11101)]
            Plain(#[deku(bits = 3)] u8, #[deku(bits = 8)] u8),
            #[deku(id_pat = "_")]
            Other(u8, #[deku(bits = 11)] u16),
        }
    };
}

instruction!(Batched, 5);
instruction!(Unbatched, "4 + 1");

/// The same instruction as `Batched` is, written as `Unbatched` is.
fn unbatched_of(value: &Batched) -> Unbatched {
    match *value {
        Batched::Addiu(ri) => Unbatched::Addiu(ri),
        Batched::Jump(jump) => Unbatched::Jump(jump),
        Batched::Load(rri) => Unbatched::Load(rri),
        Batched::Load2 { operands } => Unbatched::Load2 { operands },
        Batched::Plain(a, b) => Unbatched::Plain(a, b),
        Batched::Other(id, rest) => Unbatched::Other(id, rest),
    }
}

/// Reads `wire` both ways and checks that they agree, in value, in the
/// bits left over, in what they write back and in error.
fn assert_same(wire: &[u8]) {
    let batched = Batched::from_bytes((wire, 0));
    let unbatched = Unbatched::from_bytes((wire, 0));
    match (batched, unbatched) {
        (Ok((b_rest, b)), Ok((u_rest, u))) => {
            assert_eq!(b_rest, u_rest, "{wire:02x?}");
            assert_eq!(unbatched_of(&b), u, "{wire:02x?}");
            assert_eq!(b.to_bytes().unwrap(), u.to_bytes().unwrap(), "{wire:02x?}");
        }
        (Err(b), Err(u)) => assert_eq!(format!("{b:?}"), format!("{u:?}"), "{wire:02x?}"),
        (b, u) => panic!("batched and unbatched disagree on {wire:02x?}: {b:?} vs {u:?}"),
    }
}

#[test]
fn every_halfword_reads_as_unbatched() {
    for halfword in 0..=u16::MAX {
        // The jump takes a second halfword, so give every halfword one.
        let [a, b] = halfword.to_be_bytes();
        assert_same(&[a, b, 0x5a, 0xc3]);
    }
}

#[test]
fn the_wide_variant_reads_as_unbatched() {
    let mut x: u32 = 0x1234_5678;
    for _ in 0..20_000 {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        // Id 0b00011, then any 27 bits.
        let word = (0b00011 << 27) | (x >> 5);
        assert_same(&word.to_be_bytes());
    }
}

#[test]
fn a_read_takes_only_the_bits_of_its_variant() {
    let wire = [0b10011_101, 0b011_00111, 0xaa];
    let ((rest, offset), value) = Batched::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        value,
        Batched::Load(Rri {
            rx: 0b101,
            ry: 0b011,
            imm: 0b00111
        })
    );
    assert_eq!((rest, offset), (&[0xaa][..], 0));

    let wire = [0b00011_1_00, 0b0000_0011, 0x12, 0x34, 0xaa];
    let ((rest, offset), value) = Batched::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        value,
        Batched::Jump(Jump {
            mode: Mode::Exchange,
            high: 0b11,
            low: 0x1234
        })
    );
    assert_eq!((rest, offset), (&[0xaa][..], 0));
}

#[test]
fn a_catch_all_stores_its_id() {
    let wire = [0b00111_000, 0x05];
    let (_, value) = Batched::from_bytes((&wire, 0)).unwrap();
    assert_eq!(value, Batched::Other(0b00111, 0x05));
    assert_eq!(value.to_bytes().unwrap(), wire);
}

/// Off a byte boundary, the enum reads as it does on one.
#[test]
fn an_enum_off_a_byte_boundary() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Shifted {
        #[deku(bits = 3)]
        head: u8,
        instruction: Batched,
        #[deku(bits = 5)]
        tail: u8,
    }

    let wire = [0b101_10011, 0b101_011_00, 0b111_10101];
    let (_, value) = Shifted::from_bytes((&wire, 0)).unwrap();
    assert_eq!(
        value,
        Shifted {
            head: 0b101,
            instruction: Batched::Load(Rri {
                rx: 0b101,
                ry: 0b011,
                imm: 0b00111
            }),
            tail: 0b10101,
        }
    );
    assert_eq!(value.to_bytes().unwrap(), wire);
}

/// Without a catch-all, an id with no variant is the reader's error, once
/// the bits of the first read are there.
#[test]
fn an_unassigned_id_is_the_readers_error() {
    macro_rules! narrow {
        ($name:ident, $bits:tt) => {
            #[derive(Debug, PartialEq, DekuRead)]
            #[deku(id_type = "u8", bits = $bits, endian = "big")]
            enum $name {
                #[deku(id = 0b01)]
                A(#[deku(bits = 6)] u8),
            }
        };
    }
    narrow!(Fused, 2);
    narrow!(Unfused, "1 + 1");

    let wire = [0b11_000000];
    let fused = Fused::from_bytes((&wire, 0)).unwrap_err();
    let unfused = Unfused::from_bytes((&wire, 0)).unwrap_err();
    // The message names the enum, which is all that differs.
    assert_eq!(
        format!("{fused:?}"),
        format!("{unfused:?}").replace("Unfused", "Fused")
    );
}

/// As for the run of a struct, the documented difference: input too short
/// for the first read is `Incomplete` before the id is checked.
#[test]
fn short_input_is_incomplete_before_the_id_is_checked() {
    let wire = [0b11111_000];
    let batched = Batched::from_bytes((&wire, 0)).unwrap_err();
    assert!(matches!(batched, DekuError::Incomplete(_)), "{batched:?}");
}
