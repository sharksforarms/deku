//! Adjacent big-endian `Msb0` bit fields served by one read and one write.
//!
//! The derive batches such a run and cuts each field out with a shift and a mask.
//! The contract is that this is invisible: a batched struct must read the same
//! values, reject the same writes, and produce the same bytes as an unbatched one.
//!
//! `assert = "true"` is the lever these tests pull to get an unbatched control.
//! It disqualifies a field from a run while being semantically a no-op, so the two
//! structs below differ only in whether the derive batches them.
#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::prelude::*;

/// A realistic bit-packed header: 5 fields, 48 bits, byte-aligned overall so the
/// write side round-trips exactly. Declared twice from one definition so the two
/// cannot drift apart.
macro_rules! header {
    ($name:ident, $($extra:tt)*) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "big")]
        struct $name {
            #[deku(bits = 3 $($extra)*)]
            version: u8,
            #[deku(bits = 13 $($extra)*)]
            id: u16,
            #[deku(bits = 2 $($extra)*)]
            seq_flags: u8,
            #[deku(bits = 14 $($extra)*)]
            seq_count: u16,
            #[deku(bits = 16 $($extra)*)]
            length: u16,
        }
    };
}

// One run of 48 bits.
header!(Batched,);
// Every field disqualified, so five separate reads and writes.
header!(Unbatched, , assert = "true");

/// Deterministic pseudo-random bytes, so a failure is reproducible.
fn wire(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed;
    (0..len)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect()
}

/// The core claim: batching changes nothing observable. Every field, and the bytes
/// written back, must agree with the unbatched path over a large sample.
#[test]
fn batched_matches_unbatched() {
    for seed in 1..20_000u32 {
        let data = wire(6, seed);

        let (_, batched) = Batched::from_bytes((&data, 0)).unwrap();
        let (_, plain) = Unbatched::from_bytes((&data, 0)).unwrap();

        assert_eq!(batched.version, plain.version, "version, seed {seed}");
        assert_eq!(batched.id, plain.id, "id, seed {seed}");
        assert_eq!(batched.seq_flags, plain.seq_flags, "seq_flags, seed {seed}");
        assert_eq!(batched.seq_count, plain.seq_count, "seq_count, seed {seed}");
        assert_eq!(batched.length, plain.length, "length, seed {seed}");

        // And both must write back the bytes they came from.
        assert_eq!(batched.to_bytes().unwrap(), data, "seed {seed}");
        assert_eq!(plain.to_bytes().unwrap(), data, "seed {seed}");
    }
}

/// One wire against values computed independently of deku, so the differential
/// test above cannot pass by both paths being wrong in the same way.
#[test]
fn known_wire_yields_hand_computed_fields() {
    let data = [0xABu8, 0xCD, 0xEF, 0x12, 0x34, 0x56];
    let (_, h) = Batched::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        h,
        Batched {
            version: 5,
            id: 3021,
            seq_flags: 3,
            seq_count: 12050,
            length: 13398,
        }
    );
    assert_eq!(h.to_bytes().unwrap(), data);
}

/// A run must keep the per-field overflow rejection that the individual writes
/// performed: composing five fields into one integer must not let a too-wide value
/// silently collide with its neighbour.
#[test]
fn an_oversized_field_is_still_rejected() {
    let bad = Batched {
        version: 0b111, // fits
        id: 0xFFFF,     // 16 bits into a 13-bit field
        seq_flags: 0,
        seq_count: 0,
        length: 0,
    };
    let err = bad.to_bytes().expect_err("13-bit field cannot hold 0xFFFF");
    let msg = format!("{err}");
    assert!(
        msg.contains("bit size of input is larger than bit requested size"),
        "unexpected message: {msg}"
    );

    // The unbatched path rejects it identically.
    let plain = Unbatched {
        version: 0b111,
        id: 0xFFFF,
        seq_flags: 0,
        seq_count: 0,
        length: 0,
    };
    let plain_err = plain.to_bytes().expect_err("same field, same rejection");
    assert_eq!(format!("{plain_err}"), msg);
}

/// Widest value each field accepts, to pin the boundary rather than just the
/// overflow.
#[test]
fn each_field_accepts_its_widest_value() {
    let full = Batched {
        version: 0b111,
        id: 0x1FFF,
        seq_flags: 0b11,
        seq_count: 0x3FFF,
        length: 0xFFFF,
    };
    let bytes = full.to_bytes().unwrap();
    assert_eq!(bytes, vec![0xFF; 6]);
    let (_, back) = Batched::from_bytes((&bytes, 0)).unwrap();
    assert_eq!(back, full);
}

/// A run inside an enum variant, both named and unnamed, since those take a
/// different field-ident path in the derive.
#[test]
fn runs_inside_enum_variants() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", endian = "big")]
    enum Message {
        #[deku(id = 1)]
        Named {
            #[deku(bits = 2)]
            a: u8,
            #[deku(bits = 6)]
            b: u8,
        },
        #[deku(id = 2)]
        Unnamed(#[deku(bits = 4)] u8, #[deku(bits = 12)] u16),
    }

    let data = [1u8, 0b11_010101];
    let (_, m) = Message::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        m,
        Message::Named {
            a: 0b11,
            b: 0b010101
        }
    );
    assert_eq!(m.to_bytes().unwrap(), data);

    let data = [2u8, 0xAB, 0xCD];
    let (_, m) = Message::from_bytes((&data, 0)).unwrap();
    assert_eq!(m, Message::Unnamed(0xA, 0xBCD));
    assert_eq!(m.to_bytes().unwrap(), data);
}

/// A tuple struct: same planning, different ident path.
#[test]
fn run_in_a_tuple_struct() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Packed(#[deku(bits = 3)] u8, #[deku(bits = 13)] u16);

    let data = [0xABu8, 0xCD];
    let (_, p) = Packed::from_bytes((&data, 0)).unwrap();
    assert_eq!(p, Packed(0b101, 0x0BCD));
    assert_eq!(p.to_bytes().unwrap(), data);
}

/// A run starting part way through a struct, after a field that disqualifies
/// itself, and a second run after another.
#[test]
fn runs_around_ineligible_fields() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Mixed {
        #[deku(bits = 4)]
        a: u8,
        #[deku(bits = 4)]
        b: u8,
        #[deku(endian = "little")]
        middle: u16,
        #[deku(bits = 4)]
        c: u8,
        #[deku(bits = 4)]
        d: u8,
    }

    let data = [0x12u8, 0x34, 0x56, 0x78];
    let (_, m) = Mixed::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        m,
        Mixed {
            a: 0x1,
            b: 0x2,
            middle: 0x5634, // little-endian, so the bytes swap
            c: 0x7,
            d: 0x8,
        }
    );
    assert_eq!(m.to_bytes().unwrap(), data);
}

/// A truncated wire must still be `Incomplete`, not a partial value: one read for
/// 48 bits must not succeed on fewer.
#[test]
fn a_short_wire_still_errors() {
    for len in 0..6 {
        let data = wire(len, 5);
        assert!(
            Batched::from_bytes((&data, 0)).is_err(),
            "{len} bytes should not satisfy a 48-bit header"
        );
    }
    assert!(Batched::from_bytes((&wire(6, 5), 0)).is_ok());
}

/// `check_bit_size` is what preserves the per-field rejection, and the derive calls
/// it directly, so pin its boundaries.
#[test]
fn check_bit_size_boundaries() {
    use deku::writer::check_bit_size;

    // Widest value that fits.
    assert!(check_bit_size(0b11, 2).is_ok());
    assert!(check_bit_size(0xFF, 8).is_ok());
    assert!(check_bit_size(0, 1).is_ok());

    // One bit too wide, and the message names both widths.
    let err = check_bit_size(0b100, 2).expect_err("3 bits do not fit in 2");
    let msg = format!("{err}");
    assert!(
        msg.contains("bit size of input is larger than bit requested size"),
        "unexpected message: {msg}"
    );
    assert!(msg.contains('3') && msg.contains('2'), "message: {msg}");

    let err = check_bit_size(0x100, 8).expect_err("9 bits do not fit in 8");
    assert!(format!("{err}").contains('9'));

    // A full-width request cannot overflow, so it never errors.
    assert!(check_bit_size(u64::MAX, 64).is_ok());
}
