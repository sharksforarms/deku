//! Adjacent little-endian `Lsb0` bit fields, which one read of a
//! little-endian integer serves where the cursor is at a byte boundary.
//! `seek_from_current = "0"` is the control: it keeps a field out of a run.
#![cfg(all(feature = "alloc", feature = "bits"))]

use deku::prelude::*;

macro_rules! packed {
    ($name:ident, $($extra:tt)*) => {
        /// The unwind word of an ARM64 entry of a PE file, and a flag.
        #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
        #[deku(endian = "little", bit_order = "lsb")]
        struct $name {
            #[deku(bits = 2 $($extra)*)]
            flag: u8,
            #[deku(bits = 11 $($extra)*)]
            function_length: u16,
            #[deku(bits = 3 $($extra)*)]
            saved_float: u8,
            #[deku(bits = 4 $($extra)*)]
            saved_integer: u8,
            #[deku(bits = 1 $($extra)*)]
            homes: bool,
            #[deku(bits = 2 $($extra)*)]
            link: u8,
            #[deku(bits = 9 $($extra)*)]
            frame_size: u16,
        }
    };
}

packed!(Batched,);
packed!(Unbatched, , seek_from_current = "0");

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
fn a_run_reads_the_fields_its_own_reads_do() {
    for seed in 1..20_000u32 {
        let data = wire(4, seed);
        let (_, batched) = Batched::from_bytes((&data, 0)).unwrap();
        let (_, plain) = Unbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            (
                batched.flag,
                batched.function_length,
                batched.saved_float,
                batched.saved_integer,
                batched.homes,
                batched.link,
                batched.frame_size
            ),
            (
                plain.flag,
                plain.function_length,
                plain.saved_float,
                plain.saved_integer,
                plain.homes,
                plain.link,
                plain.frame_size
            ),
            "seed {seed}"
        );
        let word = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(u32::from(batched.flag), word & 3, "seed {seed}");
        assert_eq!(
            u32::from(batched.function_length),
            word >> 2 & 0x7ff,
            "seed {seed}"
        );
        assert_eq!(u32::from(batched.frame_size), word >> 23, "seed {seed}");
        assert_eq!(batched.to_bytes().unwrap(), data, "seed {seed}");
    }
}

#[test]
fn a_run_cut_short_is_incomplete() {
    let data = wire(4, 5);
    for len in 0..4 {
        let read = Batched::from_bytes((&data[..len], 0));
        assert!(
            matches!(read, Err(DekuError::Incomplete(_))),
            "{len} bytes: {read:?}"
        );
    }
}

/// After four bits the cursor is off a byte boundary, so the run falls back
/// to its fields' own reads, which must give the same values.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct AfterANibble {
    #[deku(bits = 4, seek_from_current = "0")]
    lead: u8,
    #[deku(bits = 6)]
    a: u8,
    #[deku(bits = 10)]
    b: u16,
    #[deku(bits = 4, seek_from_current = "0")]
    tail: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct AfterANibbleUnbatched {
    #[deku(bits = 4, seek_from_current = "0")]
    lead: u8,
    #[deku(bits = 6, seek_from_current = "0")]
    a: u8,
    #[deku(bits = 10, seek_from_current = "0")]
    b: u16,
    #[deku(bits = 4, seek_from_current = "0")]
    tail: u8,
}

#[test]
fn a_run_off_a_byte_boundary_reads_through_its_fields() {
    for seed in 1..5_000u32 {
        let data = wire(3, seed);
        let (_, batched) = AfterANibble::from_bytes((&data, 0)).unwrap();
        let (_, plain) = AfterANibbleUnbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            (batched.lead, batched.a, batched.b, batched.tail),
            (plain.lead, plain.a, plain.b, plain.tail),
            "seed {seed}"
        );
    }
}

/// A run that would end inside a byte gives up its last fields.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little", bit_order = "lsb")]
struct Uneven {
    #[deku(bits = 5)]
    a: u8,
    #[deku(bits = 11)]
    b: u16,
    #[deku(bits = 3)]
    c: u8,
    #[deku(bits = 5)]
    d: u8,
}

#[test]
fn a_run_holds_whole_bytes() {
    for seed in 1..5_000u32 {
        let data = wire(3, seed);
        let bits = u32::from_le_bytes([data[0], data[1], data[2], 0]);
        let (_, read) = Uneven::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            [
                u32::from(read.a),
                u32::from(read.b),
                u32::from(read.c),
                u32::from(read.d)
            ],
            [bits & 0x1f, bits >> 5 & 0x7ff, bits >> 16 & 7, bits >> 19],
            "seed {seed}"
        );
    }
}

#[test]
fn a_run_writes_the_bytes_its_fields_write_aligned_or_not() {
    for seed in 1..5_000u32 {
        let data = wire(3, seed);
        let (_, batched) = AfterANibble::from_bytes((&data, 0)).unwrap();
        let (_, plain) = AfterANibbleUnbatched::from_bytes((&data, 0)).unwrap();
        assert_eq!(
            batched.to_bytes().unwrap(),
            plain.to_bytes().unwrap(),
            "seed {seed}"
        );
        let (_, uneven) = Uneven::from_bytes((&data, 0)).unwrap();
        assert_eq!(uneven.to_bytes().unwrap(), data, "seed {seed}");
    }
}

#[test]
fn a_value_too_wide_for_its_bits_is_refused_as_outside_a_run() {
    let mut batched = Batched::from_bytes((&[0; 4], 0)).unwrap().1;
    let mut plain = Unbatched::from_bytes((&[0; 4], 0)).unwrap().1;
    batched.function_length = 0x800;
    plain.function_length = 0x800;
    let (batched, plain) = (batched.to_bytes(), plain.to_bytes());
    assert!(batched.is_err(), "{batched:?}");
    assert_eq!(format!("{batched:?}"), format!("{plain:?}"));
}
