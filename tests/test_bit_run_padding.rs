//! Bit fields with literal `pad_bits_before` and `pad_bits_after`, which join
//! a run of adjacent bit fields. The padding must read and write as it does
//! outside a run: dropped on the way in, and zero on the way out.
#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::prelude::*;

macro_rules! padded {
    ($name:ident, $($extra:tt)*) => {
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "big")]
        struct $name {
            #[deku(pad_bits_before = "5", bits = 3 $($extra)*)]
            first: u8,
            #[deku(bits = 6, pad_bits_after = "2" $($extra)*)]
            second: u8,
            #[deku(pad_bits_before = "1", bits = 15 $($extra)*)]
            third: u16,
        }
    };
}

padded!(Batched,);
padded!(Unbatched, , seek_from_current = "0");

/// The bits that the padding takes, which a write leaves zero.
const PAD_MASK: [u8; 4] = [0b1111_1000, 0b0000_0011, 0b1000_0000, 0];

fn wire(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed;
    (0..len)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect()
}

#[test]
fn padded_fields_read_and_write_as_outside_a_run() {
    for seed in 1..20_000u32 {
        let data = wire(4, seed);
        let (_, batched) = Batched::from_bytes((&data, 0)).unwrap();
        let (_, plain) = Unbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            (batched.first, batched.second, batched.third),
            (plain.first, plain.second, plain.third),
            "seed {seed}"
        );
        let written = batched.to_bytes().unwrap();
        assert_eq!(written, plain.to_bytes().unwrap(), "seed {seed}");
        let unpadded: Vec<u8> = data
            .iter()
            .zip(PAD_MASK)
            .map(|(byte, pad)| byte & !pad)
            .collect();
        assert_eq!(written, unpadded, "seed {seed}");
    }
}

#[test]
fn short_input_is_incomplete() {
    let data = wire(4, 3);
    for len in 0..4 {
        let batched = Batched::from_bytes((&data[..len], 0));
        assert!(
            matches!(batched, Err(DekuError::Incomplete(_))),
            "{len} bytes: {batched:?}"
        );
    }
}

/// After 7 bits the cursor is not aligned to a byte, so the run of 9 bits
/// starts in the middle of one.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Unaligned {
    #[deku(bits = 3)]
    lead: u8,
    #[deku(bits = 4, seek_from_current = "0")]
    split: u8,
    #[deku(pad_bits_before = "2", bits = 1)]
    flag: u8,
    #[deku(bits = 3, pad_bits_after = "3")]
    tail: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct UnalignedUnbatched {
    #[deku(bits = 3)]
    lead: u8,
    #[deku(bits = 4, seek_from_current = "0")]
    split: u8,
    #[deku(pad_bits_before = "2", bits = 1, seek_from_current = "0")]
    flag: u8,
    #[deku(bits = 3, pad_bits_after = "3", seek_from_current = "0")]
    tail: u8,
}

#[test]
fn a_padded_run_on_an_unaligned_cursor_reads_the_same_bits() {
    for seed in 1..5_000u32 {
        let data = wire(2, seed);
        let (_, batched) = Unaligned::from_bytes((&data, 0)).unwrap();
        let (_, plain) = UnalignedUnbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            (batched.lead, batched.split, batched.flag, batched.tail),
            (plain.lead, plain.split, plain.flag, plain.tail),
            "seed {seed}"
        );
        assert_eq!(
            batched.to_bytes().unwrap(),
            plain.to_bytes().unwrap(),
            "seed {seed}"
        );
    }
}

/// A skip of a few bits, as `pad_bits_before` on a field outside a run makes,
/// leaves the reader where `read_bits_into` would.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Skips {
    #[deku(bits = 3)]
    lead: u8,
    #[deku(pad_bits_before = "3", seek_from_current = "0")]
    byte: u8,
    #[deku(pad_bits_before = "8", bits = 2, seek_from_current = "0")]
    tail: u8,
}

#[test]
fn a_skip_of_a_few_bits_leaves_the_reader_where_a_read_would() {
    for seed in 1..5_000u32 {
        let data = wire(3, seed);
        let (rest, skips) = Skips::from_bytes((&data, 0)).unwrap();
        let bits: u32 = u32::from_be_bytes([0, data[0], data[1], data[2]]);
        assert_eq!(u32::from(skips.lead), bits >> 21, "seed {seed}");
        assert_eq!(u32::from(skips.byte), bits >> 10 & 0xff, "seed {seed}");
        assert_eq!(u32::from(skips.tail), bits & 0b11, "seed {seed}");
        assert_eq!(rest, (&[][..], 0), "seed {seed}");
    }
}
