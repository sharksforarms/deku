//! Adjacent big-endian `Msb0` bit fields served by one read and one write.
//!
//! Batching must be invisible: same values, same rejections, same bytes as an
//! unbatched struct. `assert = "true"` is the control, since it disqualifies a
//! field from a run while being a no-op.
#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::prelude::*;

/// A bit-packed header: 5 fields, 48 bits. Declared twice from one definition so
/// the two cannot drift apart.
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

/// The core claim: every field and the bytes written back agree with unbatched.
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

/// One wire against independently computed values, in case both paths are wrong.
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

/// Composing fields into one integer must not let a too-wide value collide with
/// its neighbour.
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
    let msg = format!("{err:?}");
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
    assert_eq!(format!("{plain_err:?}"), msg);
}

/// The boundary, not just the overflow.
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

/// Enum variants, named and unnamed: a different field-ident path.
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

/// Runs either side of an ineligible field.
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

/// One read for 48 bits must not succeed on fewer.
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

/// `check_bit_size` preserves the per-field rejection, so pin its boundaries.
#[test]
fn check_bit_size_boundaries() {
    use deku::writer::check_bit_size;

    // Widest value that fits.
    assert!(check_bit_size::<false>(0b11, 2).is_ok());
    assert!(check_bit_size::<false>(0xFF, 8).is_ok());
    assert!(check_bit_size::<false>(0, 1).is_ok());
    // A full-width request cannot overflow, so it never errors.
    assert!(check_bit_size::<false>(u64::MAX, 64).is_ok());

    // One bit too wide, and the message names both widths.
    let err = check_bit_size::<false>(0b100, 2).expect_err("3 bits do not fit in 2");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("bit size of input is larger than bit requested size"),
        "unexpected message: {msg}"
    );
    // Only `descriptive-errors` appends the two widths.
    #[cfg(feature = "descriptive-errors")]
    {
        assert!(msg.contains('3') && msg.contains('2'), "message: {msg}");
        let err = check_bit_size::<false>(0x100, 8).expect_err("9 bits do not fit in 8");
        assert!(format!("{err:?}").contains('9'));
    }

    // Same verdict either way; only the wording differs.
    for (value, bits) in [(0b11u64, 2), (0xFF, 8), (0, 1), (u64::MAX, 64), (0b100, 2)] {
        assert_eq!(
            check_bit_size::<false>(value, bits).is_ok(),
            check_bit_size::<true>(value, bits).is_ok(),
            "{value:#x} in {bits} bits"
        );
    }
    let ordered = format!("{:?}", check_bit_size::<true>(0b100, 2).unwrap_err());
    assert!(
        ordered.contains("bit size of input is larger than requested size")
            && !ordered.contains("bit requested size"),
        "unexpected message: {ordered}"
    );
}

/// Flags as bools, shaped like the RFC 3550 fixed header: 64 bits, three bools.
macro_rules! flags_header {
    ($name:ident, $($extra:tt)*) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "big")]
        struct $name {
            #[deku(bits = 2 $($extra)*)]
            version: u8,
            #[deku(bits = 1 $($extra)*)]
            padding: bool,
            #[deku(bits = 1 $($extra)*)]
            extension: bool,
            #[deku(bits = 4 $($extra)*)]
            csrc_count: u8,
            #[deku(bits = 1 $($extra)*)]
            marker: bool,
            #[deku(bits = 7 $($extra)*)]
            payload_type: u8,
            #[deku(bits = 16 $($extra)*)]
            sequence_number: u16,
            #[deku(bits = 32 $($extra)*)]
            timestamp: u32,
        }
    };
}

flags_header!(FlagsBatched,);
flags_header!(FlagsUnbatched, , assert = "true");

/// A one-bit bool has no invalid value, so every wire must agree with unbatched.
#[test]
fn a_header_of_flags_matches_unbatched() {
    for seed in 1..20_000u32 {
        let data = wire(8, seed);

        let (_, b) = FlagsBatched::from_bytes((&data, 0)).unwrap();
        let (_, p) = FlagsUnbatched::from_bytes((&data, 0)).unwrap();

        assert_eq!(b.version, p.version, "version, seed {seed}");
        assert_eq!(b.padding, p.padding, "padding, seed {seed}");
        assert_eq!(b.extension, p.extension, "extension, seed {seed}");
        assert_eq!(b.csrc_count, p.csrc_count, "csrc_count, seed {seed}");
        assert_eq!(b.marker, p.marker, "marker, seed {seed}");
        assert_eq!(b.payload_type, p.payload_type, "payload_type, seed {seed}");
        assert_eq!(
            b.sequence_number, p.sequence_number,
            "sequence_number, seed {seed}"
        );
        assert_eq!(b.timestamp, p.timestamp, "timestamp, seed {seed}");

        assert_eq!(b.to_bytes().unwrap(), data, "seed {seed}");
        assert_eq!(p.to_bytes().unwrap(), data, "seed {seed}");
    }
}

/// Both flag states, written back exactly.
#[test]
fn flags_round_trip_in_both_states() {
    let all_set = [0xFFu8; 8];
    let (_, b) = FlagsBatched::from_bytes((&all_set, 0)).unwrap();
    assert!(b.padding && b.extension && b.marker);
    assert_eq!(b.to_bytes().unwrap(), all_set);

    let none_set = [0x00u8; 8];
    let (_, b) = FlagsBatched::from_bytes((&none_set, 0)).unwrap();
    assert!(!b.padding && !b.extension && !b.marker);
    assert_eq!(b.to_bytes().unwrap(), none_set);
}

/// A bool without `bits` is a byte, so it has invalid values to reject.
#[test]
fn a_byte_wide_bool_in_a_run_rejects_a_non_boolean_value() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Batched {
        flag: bool,
        other: u8,
    }

    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Unbatched {
        #[deku(assert = "true")]
        flag: bool,
        other: u8,
    }

    // 0 and 1 are the only accepted encodings, and round-trip.
    for (byte, expected) in [(0x00u8, false), (0x01, true)] {
        let data = [byte, 0x42];
        let (_, b) = Batched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            b,
            Batched {
                flag: expected,
                other: 0x42
            }
        );
        assert_eq!(b.to_bytes().unwrap(), data);
    }

    // Anything else fails, with the same error the unbatched path gives.
    for byte in [0x02u8, 0x7F, 0xFF] {
        let data = [byte, 0x42];
        let b = Batched::from_bytes((&data, 0)).expect_err("not a bool");
        let p = Unbatched::from_bytes((&data, 0)).expect_err("not a bool");
        assert_eq!(format!("{b:?}"), format!("{p:?}"), "byte {byte:#04x}");
        assert!(
            format!("{b:?}").contains("cannot parse bool value"),
            "unexpected message: {b}"
        );
    }
}

/// `bit_order = "msb"` is the default spelled out, so it must batch. It takes the
/// `Order`-carrying impl, whose overflow wording the batched write must follow.
#[test]
fn an_explicit_msb_bit_order_batches() {
    macro_rules! ordered_header {
        ($name:ident, $($extra:tt)*) => {
            #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
            #[deku(endian = "big")]
            struct $name {
                #[deku(bits = 3, bit_order = "msb" $($extra)*)]
                a: u8,
                #[deku(bits = 13, bit_order = "msb" $($extra)*)]
                b: u16,
            }
        };
    }

    ordered_header!(OrderedBatched,);
    ordered_header!(OrderedUnbatched, , assert = "true");

    for seed in 1..2_000u32 {
        let data = wire(2, seed);
        let (_, x) = OrderedBatched::from_bytes((&data, 0)).unwrap();
        let (_, y) = OrderedUnbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!((x.a, x.b), (y.a, y.b), "seed {seed}");
        assert_eq!(x.to_bytes().unwrap(), data, "seed {seed}");
    }

    let batched = format!(
        "{:?}",
        OrderedBatched { a: 0, b: 0xFFFF }.to_bytes().unwrap_err()
    );
    let unbatched = format!(
        "{:?}",
        OrderedUnbatched { a: 0, b: 0xFFFF }.to_bytes().unwrap_err()
    );
    assert_eq!(batched, unbatched);
    assert!(
        batched.contains("bit size of input is larger than requested size"),
        "unexpected message: {batched}"
    );
    assert!(
        !batched.contains("bit requested size"),
        "used the default impl's wording: {batched}"
    );
}

/// A run may mix the two, and each field keeps its own wording.
#[test]
fn a_mixed_run_keeps_each_fields_wording() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Mixed {
        #[deku(bits = 4, bit_order = "msb")]
        ordered: u8,
        #[deku(bits = 4)]
        plain: u8,
    }

    // Both fields still share one read and one write.
    let data = [0x12u8];
    let (_, m) = Mixed::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        m,
        Mixed {
            ordered: 0x1,
            plain: 0x2
        }
    );
    assert_eq!(m.to_bytes().unwrap(), data);

    let ordered_err = format!(
        "{:?}",
        Mixed {
            ordered: 0xFF,
            plain: 0
        }
        .to_bytes()
        .unwrap_err()
    );
    assert!(
        ordered_err.contains("bit size of input is larger than requested size")
            && !ordered_err.contains("bit requested size"),
        "unexpected message: {ordered_err}"
    );

    let plain_err = format!(
        "{:?}",
        Mixed {
            ordered: 0,
            plain: 0xFF
        }
        .to_bytes()
        .unwrap_err()
    );
    assert!(
        plain_err.contains("bit size of input is larger than bit requested size"),
        "unexpected message: {plain_err}"
    );
}
