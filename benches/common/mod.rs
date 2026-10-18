//! Shapes shared by the `bebits` benches, which measure the same structs under
//! two harnesses: criterion for wall time, gungraun/callgrind for instruction
//! counts. The definitions live here so that the two harnesses cannot drift
//! apart and report numbers for different shapes.
//!
//! Big-endian bit-packed headers: the shape used by real network / space
//! protocols (CCSDS, IPv4, DVB-S2). Mirrors the CCSDS TM Transfer Frame
//! primary header, 6 octets / 11 fields.

// Each bench target compiles this module on its own and uses a part of it, so
// the items the other target uses look dead here.
#![allow(dead_code)]

use deku::prelude::*;
use no_std_io::io::{Cursor, Read, Seek};

/// Frames per sequential pass. 128 six-octet frames is 768 bytes, comfortably
/// inside L1 so the measurement is field decoding rather than memory.
pub const FRAMES: usize = 128;

/// 1-bit fields per sequential pass: 1024 bits, i.e. 128 bytes.
pub const BITS: usize = 1024;

/// Bytes that decode into one [`TmPrimaryHeader`]. The trailing two octets let a
/// bench read past the 6-octet header without hitting the end of the cursor.
pub const HEADER_BYTES: [u8; 8] = [0x2A, 0xB5, 0x11, 0x22, 0xC7, 0xFF, 0x00, 0x99];

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct TmPrimaryHeader {
    #[deku(bits = 2)]
    pub tfvn: u8,
    #[deku(bits = 10)]
    pub scid: u16,
    #[deku(bits = 3)]
    pub vcid: u8,
    #[deku(bits = 1)]
    pub ocf: u8,
    pub mcfc: u8,
    pub vcfc: u8,
    #[deku(bits = 1)]
    pub tfs: u8,
    #[deku(bits = 1)]
    pub syn: u8,
    #[deku(bits = 1)]
    pub po: u8,
    #[deku(bits = 2)]
    pub sli: u8,
    #[deku(bits = 11)]
    pub fhp: u16,
}

/// Same 6 octets, byte-aligned: the deku fast path, for scale. Also the control
/// for any change to the bit paths, which must leave this one alone.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct SixBytes {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
}

/// Worst case in the docs: a 1-bit field in a wide container.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct OneBitU64 {
    #[deku(bits = 1)]
    pub a: u64,
}

/// Two 4-byte address fields, the shape `[u8; N]` makes expensive: the generic
/// array impl reads them one element at a time. Same layout as the IPv4 source
/// and destination addresses.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Addresses {
    pub source: [u8; 4],
    pub destination: [u8; 4],
}

/// 8-octet items that fit in the same stream as the 6-octet frames.
pub const ADDRS: usize = FRAMES * 6 / 8;

/// Fills `buf` with a frame stream whose bytes are not compile-time constants.
/// A linear congruential generator keeps the bytes identical between harnesses
/// and between runs, so the two report numbers for the same input.
fn fill(buf: &mut [u8]) {
    let mut x: u32 = 0x1234_5678;
    for b in buf.iter_mut() {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *b = (x >> 24) as u8;
    }
}

/// A frame stream on the stack, for harnesses that borrow their input.
pub fn stream_array() -> [u8; FRAMES * 6] {
    let mut buf = [0u8; FRAMES * 6];
    fill(&mut buf);
    buf
}

/// A frame stream that owns its bytes, for harnesses that take their input by
/// value.
#[cfg(feature = "alloc")]
pub fn stream_vec() -> Vec<u8> {
    let mut buf = vec![0u8; FRAMES * 6];
    fill(&mut buf);
    buf
}

/// Decodes one `T` from `reader`.
pub fn read<T: for<'a> DekuContainerRead<'a>>(mut reader: impl Read + Seek) -> T {
    let mut reader = Reader::new(&mut reader);
    T::from_reader_with_ctx(&mut reader, ()).unwrap()
}

/// The header the write benches emit. Decoded from the bytes the read benches
/// use, so that both sides measure the same field values.
pub fn header() -> TmPrimaryHeader {
    read(Cursor::new(&HEADER_BYTES[..]))
}

/// The `SixBytes` value the write benches emit.
pub fn six() -> SixBytes {
    SixBytes {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: 5,
        f: 6,
    }
}

/// The `Addresses` value the write benches emit.
pub fn addresses() -> Addresses {
    Addresses {
        source: [10, 0, 0, 1],
        destination: [192, 168, 1, 254],
    }
}
