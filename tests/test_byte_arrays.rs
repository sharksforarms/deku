//! `[u8; N]` fields served by one `read_exact` instead of one read per element.
//!
//! The contract is that the shortcut is invisible: it must produce exactly what
//! the generic `[T; N]` impl produces, at every bit offset.

#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::ctx::{Endian, Order};
use deku::no_std_io::Cursor;
use deku::prelude::*;

/// What the generic `[T; N]` impl yields for the same input: N sequential byte
/// reads. This is the oracle the derived read must match.
fn generic_array<const N: usize>(wire: &[u8], skip: usize) -> Result<[u8; N], DekuError> {
    let mut cursor = Cursor::new(wire);
    let mut reader = Reader::new(&mut cursor);
    if skip != 0 {
        reader.skip_bits(skip, Order::Msb0)?;
    }
    <[u8; N] as DekuReader<'_, Endian>>::from_reader_with_ctx(&mut reader, Endian::Big)
}

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

/// One struct per leading-bit-field width, so the array starts at every bit
/// offset. `head` and `tail` sum to 8 bits, keeping the struct byte-aligned.
macro_rules! offset_case {
    ($name:ident, $head:literal, $tail:literal, $n:literal) => {
        #[test]
        fn $name() {
            #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
            #[deku(endian = "big")]
            struct S {
                #[deku(bits = $head)]
                head: u8,
                body: [u8; $n],
                #[deku(bits = $tail)]
                tail: u8,
            }

            for seed in 1..200u32 {
                let data = wire($n + 1, seed);
                let (_, s) = S::from_bytes((&data, 0)).unwrap();

                // The array must hold exactly what the generic impl would read
                // at that offset.
                let expected = generic_array::<$n>(&data, $head).unwrap();
                assert_eq!(s.body, expected, "seed {seed}, offset {}", $head);

                // And the whole struct must write back byte for byte.
                assert_eq!(s.to_bytes().unwrap(), data, "seed {seed}");
            }
        }
    };
}

offset_case!(offset_1_bit, 1, 7, 4);
offset_case!(offset_2_bits, 2, 6, 4);
offset_case!(offset_3_bits, 3, 5, 4);
offset_case!(offset_4_bits, 4, 4, 4);
offset_case!(offset_5_bits, 5, 3, 4);
offset_case!(offset_6_bits, 6, 2, 4);
offset_case!(offset_7_bits, 7, 1, 4);
offset_case!(offset_wide_array, 3, 5, 20);

/// The aligned case, which is the one that matters for throughput.
#[test]
fn byte_aligned_array() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Addresses {
        source: [u8; 4],
        destination: [u8; 4],
    }

    let data = wire(8, 7);
    let (_, a) = Addresses::from_bytes((&data, 0)).unwrap();
    assert_eq!(a.source, generic_array::<4>(&data, 0).unwrap());
    assert_eq!(a.destination, generic_array::<4>(&data[4..], 0).unwrap());
    assert_eq!(a.to_bytes().unwrap(), data);
}

/// A short buffer must still be `Incomplete`, not a panic or a truncated value.
#[test]
fn truncated_input_still_errors() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct S {
        body: [u8; 8],
    }

    for len in 0..8 {
        let data = wire(len, 3);
        assert!(
            S::from_bytes((&data, 0)).is_err(),
            "{len} bytes should not satisfy an 8-byte array"
        );
    }
    assert!(S::from_bytes((&wire(8, 3), 0)).is_ok());
}

/// A byte array inside an enum variant takes the same path.
#[test]
fn array_in_enum_variant() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", endian = "big")]
    enum Message {
        #[deku(id = 1)]
        Addr { octets: [u8; 4] },
        #[deku(id = 2)]
        Pair([u8; 2], u8),
    }

    let data = [1u8, 0xDE, 0xAD, 0xBE, 0xEF];
    let (_, m) = Message::from_bytes((&data, 0)).unwrap();
    assert_eq!(
        m,
        Message::Addr {
            octets: [0xDE, 0xAD, 0xBE, 0xEF]
        }
    );
    assert_eq!(m.to_bytes().unwrap(), data);

    let data = [2u8, 0x11, 0x22, 0x33];
    let (_, m) = Message::from_bytes((&data, 0)).unwrap();
    assert_eq!(m, Message::Pair([0x11, 0x22], 0x33));
    assert_eq!(m.to_bytes().unwrap(), data);
}

/// Fields the shortcut must decline, each for a different reason.
#[test]
fn declined_shapes_are_unchanged() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    struct Declined {
        // An element type that is not a byte.
        words: [u16; 2],
        // `bits` changes what each element is.
        #[deku(bits = 4)]
        nibble: u8,
        #[deku(bits = 4)]
        other: u8,
        // Padding moves the cursor around the field.
        #[deku(pad_bytes_before = "1")]
        padded: [u8; 2],
        // A map rewrites the value.
        #[deku(map = "|v: [u8; 2]| -> Result<_, DekuError> { Ok([v[1], v[0]]) }")]
        swapped: [u8; 2],
    }

    let data = [0xAA, 0xBB, 0xCC, 0xDD, 0x12, 0x00, 0x34, 0x56, 0x78, 0x9A];
    let (_, d) = Declined::from_bytes((&data, 0)).unwrap();
    assert_eq!(d.words, [0xAABB, 0xCCDD]);
    assert_eq!(d.nibble, 0x1);
    assert_eq!(d.other, 0x2);
    assert_eq!(d.padded, [0x34, 0x56]);
    assert_eq!(d.swapped, [0x9A, 0x78]);
}

/// `update` is excused, as it is for a bit run: it feeds `DekuUpdate::update`
/// only, never the read or the write.
#[test]
fn update_is_excused_and_still_applies() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    struct Updating {
        len: u8,
        #[deku(update = "[0xEE; 2]")]
        body: [u8; 2],
    }

    let data = [0x02, 0x11, 0x22];
    let (_, mut u) = Updating::from_bytes((&data, 0)).unwrap();

    // The read is untouched by `update`, and the write emits what was read.
    assert_eq!(u.body, [0x11, 0x22]);
    assert_eq!(u.to_bytes().unwrap(), data);

    // `update` assigns, and the shortcut then writes the assigned value.
    u.update().unwrap();
    assert_eq!(u.body, [0xEE, 0xEE]);
    assert_eq!(u.to_bytes().unwrap(), [0x02, 0xEE, 0xEE]);
}

/// A little-endian container: byte order cannot affect a byte array, so the
/// shortcut applies and must agree with the generic impl.
#[test]
fn little_endian_container() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "little")]
    struct S {
        body: [u8; 4],
        trailer: u16,
    }

    let data = [1u8, 2, 3, 4, 0x34, 0x12];
    let (_, s) = S::from_bytes((&data, 0)).unwrap();
    assert_eq!(s.body, [1, 2, 3, 4]);
    assert_eq!(s.trailer, 0x1234);
    assert_eq!(s.to_bytes().unwrap(), data);
}

/// A byte array whose write begins on a partial `Lsb0` leftover, which reorders
/// spliced bytes, so the shortcut has to decline. Whichever field came before
/// leaves the leftover, so the array's own `bit_order` says nothing about it.
#[test]
fn array_after_a_partial_lsb0_field() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    struct Shortcut {
        #[deku(bits = 3, bit_order = "lsb")]
        head: u8,
        body: [u8; 2],
    }

    // The same layout with the array spelled out as its elements, which is what
    // the generic `[T; N]` impl does: N sequential byte writes. The oracle.
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    struct Oracle {
        #[deku(bits = 3, bit_order = "lsb")]
        head: u8,
        b0: u8,
        b1: u8,
    }

    for seed in 1..200u32 {
        let data = wire(3, seed);
        let (_, s) = Shortcut::from_bytes((&data, 0)).unwrap();
        let (_, o) = Oracle::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            (s.head, s.body),
            (o.head, [o.b0, o.b1]),
            "read differs, seed {seed}"
        );

        // The bytes must match what the elements would have written. Reading
        // those bytes back is not asserted: an `Lsb0` field followed by `Msb0`
        // bytes does not round-trip in deku today, with or without the shortcut.
        assert_eq!(
            s.to_bytes().unwrap(),
            o.to_bytes().unwrap(),
            "write differs, seed {seed}"
        );
    }
}
