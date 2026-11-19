//! Adjacent whole-byte integer fields served by one byte read.
//!
//! Batching must be invisible: the same values and the same rejections as an
//! unbatched struct. `seek_from_current = "0"` is the control, since it keeps a field out
//! of a run while being a no-op.
#![cfg(feature = "alloc")]

use deku::ctx::Endian;
use deku::prelude::*;

/// A header of mixed widths and signs, declared twice from one definition so
/// the two cannot drift apart.
macro_rules! header {
    ($name:ident, $endian:literal, $($extra:tt)*) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = $endian)]
        struct $name {
            #[deku($($extra)*)]
            kind: u8,
            #[deku($($extra)*)]
            length: u16,
            #[deku($($extra)*)]
            offset: i32,
            #[deku($($extra)*)]
            address: u64,
            #[deku($($extra)*)]
            delta: i8,
            #[deku($($extra)*)]
            id: u128,
        }
    };
}

header!(BigBatched, "big",);
header!(BigUnbatched, "big", seek_from_current = "0");
header!(LittleBatched, "little",);
header!(LittleUnbatched, "little", seek_from_current = "0");

/// The number of bytes the header takes.
const HEADER_BYTES: usize = 1 + 2 + 4 + 8 + 1 + 16;

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

macro_rules! same_fields {
    ($batched:expr, $plain:expr, $seed:expr) => {
        let (batched, plain) = ($batched, $plain);
        assert_eq!(batched.kind, plain.kind, "kind, seed {}", $seed);
        assert_eq!(batched.length, plain.length, "length, seed {}", $seed);
        assert_eq!(batched.offset, plain.offset, "offset, seed {}", $seed);
        assert_eq!(batched.address, plain.address, "address, seed {}", $seed);
        assert_eq!(batched.delta, plain.delta, "delta, seed {}", $seed);
        assert_eq!(batched.id, plain.id, "id, seed {}", $seed);
    };
}

#[test]
fn batched_matches_unbatched_in_each_byte_order() {
    for seed in 1..5_000u32 {
        let data = wire(HEADER_BYTES, seed);

        let (_, big) = BigBatched::from_bytes((&data, 0)).unwrap();
        let (_, plain) = BigUnbatched::from_bytes((&data, 0)).unwrap();
        same_fields!(&big, &plain, seed);
        assert_eq!(big.to_bytes().unwrap(), data, "seed {seed}");

        let (_, little) = LittleBatched::from_bytes((&data, 0)).unwrap();
        let (_, plain) = LittleUnbatched::from_bytes((&data, 0)).unwrap();
        same_fields!(&little, &plain, seed);
        assert_eq!(little.to_bytes().unwrap(), data, "seed {seed}");
    }
}

#[test]
fn known_wire_yields_hand_computed_fields() {
    let data: Vec<u8> = (1..=HEADER_BYTES as u8).collect();
    let (_, big) = BigBatched::from_bytes((&data, 0)).unwrap();
    assert_eq!(big.kind, 0x01);
    assert_eq!(big.length, 0x0203);
    assert_eq!(big.offset, 0x0405_0607);
    assert_eq!(big.address, 0x0809_0a0b_0c0d_0e0f);
    assert_eq!(big.delta, 0x10);
    assert_eq!(big.id, 0x1112_1314_1516_1718_191a_1b1c_1d1e_1f20);

    let (_, little) = LittleBatched::from_bytes((&data, 0)).unwrap();
    assert_eq!(little.length, 0x0302);
    assert_eq!(little.offset, 0x0706_0504);
}

#[test]
fn short_input_is_incomplete() {
    let data = wire(HEADER_BYTES, 7);
    for len in 0..HEADER_BYTES {
        let batched = BigBatched::from_bytes((&data[..len], 0));
        let plain = BigUnbatched::from_bytes((&data[..len], 0));
        assert!(
            matches!(batched, Err(DekuError::Incomplete(_))),
            "{len} bytes: {batched:?}"
        );
        assert!(
            matches!(plain, Err(DekuError::Incomplete(_))),
            "{len} bytes"
        );
    }
}

/// A byte order from `ctx`, which is known only at run time.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: Endian")]
struct Words {
    first: u32,
    second: u16,
}

#[test]
fn a_run_reads_in_the_byte_order_of_its_ctx() {
    let data = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];
    for (endian, expected) in [
        (
            Endian::Big,
            Words {
                first: 0x1234_5678,
                second: 0x9abc,
            },
        ),
        (
            Endian::Little,
            Words {
                first: 0x7856_3412,
                second: 0xbc9a,
            },
        ),
    ] {
        let mut cursor = deku::no_std_io::Cursor::new(&data);
        let mut reader = Reader::new(&mut cursor);
        let words = Words::from_reader_with_ctx(&mut reader, endian).unwrap();
        assert_eq!(words, expected);
    }
}

/// With no `endian`, a field reads in the target's byte order, and a run must
/// do the same.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct TargetOrder {
    first: u16,
    second: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct TargetOrderUnbatched {
    #[deku(seek_from_current = "0")]
    first: u16,
    #[deku(seek_from_current = "0")]
    second: u32,
}

#[test]
fn a_run_with_no_endian_reads_in_the_targets_byte_order() {
    let data = [1, 2, 3, 4, 5, 6];
    let (_, batched) = TargetOrder::from_bytes((&data, 0)).unwrap();
    let (_, plain) = TargetOrderUnbatched::from_bytes((&data, 0)).unwrap();
    assert_eq!((batched.first, batched.second), (plain.first, plain.second));
}

/// A field with an endian of its own ends the run before it.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct MixedOrder {
    first: u16,
    second: u16,
    #[deku(endian = "little")]
    third: u16,
    fourth: u16,
}

#[test]
fn a_field_of_another_byte_order_splits_the_run() {
    let data = [0x00, 0x01, 0x00, 0x02, 0x03, 0x00, 0x00, 0x04];
    let (_, mixed) = MixedOrder::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        mixed,
        MixedOrder {
            first: 1,
            second: 2,
            third: 3,
            fourth: 4
        }
    );
}

/// After a field of a few bits the cursor is not aligned to a byte, and the
/// run must read the same bits the fields' own reads would.
#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Unaligned {
    #[deku(bits = 4)]
    flags: u8,
    first: u16,
    second: u8,
    #[deku(bits = 4)]
    tail: u8,
}

#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct UnalignedUnbatched {
    #[deku(bits = 4)]
    flags: u8,
    #[deku(seek_from_current = "0")]
    first: u16,
    #[deku(seek_from_current = "0")]
    second: u8,
    #[deku(bits = 4)]
    tail: u8,
}

#[cfg(feature = "bits")]
#[test]
fn a_run_on_an_unaligned_cursor_reads_the_same_bits() {
    for seed in 1..2_000u32 {
        let data = wire(4, seed);
        let (_, batched) = Unaligned::from_bytes((&data, 0)).unwrap();
        let (_, plain) = UnalignedUnbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            (batched.flags, batched.first, batched.second, batched.tail),
            (plain.flags, plain.first, plain.second, plain.tail),
            "seed {seed}"
        );
    }
}

/// The fields of an enum variant form a run too, after the id.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "big")]
enum Message {
    #[deku(id = 1)]
    Point { x: i32, y: i32 },
    #[deku(id = 2)]
    Pair(u16, u16),
}

#[test]
fn the_fields_of_an_enum_variant_form_a_run() {
    let data = [1, 0xff, 0xff, 0xff, 0xfe, 0, 0, 0, 3];
    let (_, point) = Message::from_bytes((&data, 0)).unwrap();
    assert_eq!(point, Message::Point { x: -2, y: 3 });
    let (_, pair) = Message::from_bytes((&[2, 0, 5, 0, 6][..], 0)).unwrap();
    assert_eq!(pair, Message::Pair(5, 6));
    assert_eq!(pair.to_bytes().unwrap(), [2, 0, 5, 0, 6]);
}

/// A later field can name the fields a run read.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
struct Counted {
    tag: u16,
    count: u16,
    #[deku(count = "count")]
    items: Vec<u8>,
}

#[test]
fn a_later_field_reads_the_fields_of_a_run() {
    let data = [7, 0, 3, 0, 9, 8, 7];
    let (_, counted) = Counted::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        counted,
        Counted {
            tag: 7,
            count: 3,
            items: vec![9, 8, 7]
        }
    );
}
