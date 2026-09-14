//! A multi-protocol pipeline: several packet shapes, decoded from one stream in
//! an order the branch predictor cannot learn.
//!
//! The single-shape benches in `benches/common/mod.rs` measure one struct in a
//! tight loop. That is the case a reader change flatters: the same field widths
//! every iteration, one branch target, a hot instruction cache. Real parsers
//! instead dispatch on a type byte and decode whichever shape arrives next.
//!
//! These shapes are deliberately narrow-field heavy. `Reader::read_bits_uint_msb0`
//! reads whole bytes only when the leftover cannot satisfy the request, so a
//! stream of 1- to 12-bit fields asks it for one byte at a time. That is the
//! case a wide `read_exact` cannot help and can only slow down, and it is the
//! case the comment above that loop says regressed.

use deku::prelude::*;

/// Packets per pass. Enough to defeat the branch predictor, small enough that
/// the whole stream stays in L1.
pub const PACKETS: usize = 256;

/// A CCSDS space packet primary header: 6 octets, 7 fields, all narrow.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct SpacePacket {
    #[deku(bits = 3)]
    pub version: u8,
    #[deku(bits = 1)]
    pub pkt_type: u8,
    #[deku(bits = 1)]
    pub sec_hdr: u8,
    #[deku(bits = 11)]
    pub apid: u16,
    #[deku(bits = 2)]
    pub seq_flags: u8,
    #[deku(bits = 14)]
    pub seq_count: u16,
    pub length: u16,
}

/// An IPv4 header without options: mixed narrow fields and whole bytes, the
/// shape the crate's own example uses.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Ipv4Header {
    #[deku(bits = 4)]
    pub version: u8,
    #[deku(bits = 4)]
    pub ihl: u8,
    #[deku(bits = 6)]
    pub dscp: u8,
    #[deku(bits = 2)]
    pub ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    #[deku(bits = 3)]
    pub flags: u8,
    #[deku(bits = 13)]
    pub frag_offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src: u32,
    pub dst: u32,
}

/// A DVB-S2 BBFRAME header: 10 octets, a mix of flag bits and whole fields.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct BbHeader {
    #[deku(bits = 2)]
    pub ts_gs: u8,
    #[deku(bits = 1)]
    pub sis_mis: u8,
    #[deku(bits = 1)]
    pub ccm_acm: u8,
    #[deku(bits = 1)]
    pub issyi: u8,
    #[deku(bits = 1)]
    pub npd: u8,
    #[deku(bits = 2)]
    pub ro: u8,
    pub isi: u8,
    pub upl: u16,
    pub dfl: u16,
    pub sync: u8,
    pub syncd: u16,
    pub crc: u8,
}

/// A byte-aligned record with no bit fields: the control. A change to the bit
/// paths must leave this one alone.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Telemetry {
    pub sensor: u8,
    pub seq: u8,
    pub value: u32,
    pub flags: u16,
}

/// The tagged union a dispatch loop decodes. The id byte makes the shape of the
/// next packet unknown until it is read, which is the point.
// No `endian` here: each inner struct declares its own, and an `endian` on the
// enum is forwarded as a context to variants that do not take one.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
pub enum Packet {
    #[deku(id = 0x01)]
    Space(SpacePacket),
    #[deku(id = 0x02)]
    Ipv4(Ipv4Header),
    #[deku(id = 0x03)]
    Bb(BbHeader),
    #[deku(id = 0x04)]
    Telem(Telemetry),
}

impl Packet {
    /// One field from each shape, folded into the accumulator so that no decode
    /// can be dropped.
    pub fn fold(&self) -> u64 {
        match self {
            Packet::Space(p) => u64::from(p.apid) ^ u64::from(p.seq_count),
            Packet::Ipv4(p) => u64::from(p.src) ^ u64::from(p.total_length),
            Packet::Bb(p) => u64::from(p.dfl) ^ u64::from(p.upl),
            Packet::Telem(p) => u64::from(p.value) ^ u64::from(p.flags),
        }
    }
}

/// Decodes one packet, so that both harnesses share one definition of the call.
pub fn read_packet<R: no_std_io::io::Read + no_std_io::io::Seek>(reader: &mut Reader<R>) -> Packet {
    Packet::from_reader_with_ctx(reader, ()).unwrap()
}

/// Bytes for one packet of each kind, tagged, in a fixed rotation.
///
/// The rotation is 1,2,3,4,1,3,2,4,... rather than a plain cycle: a strict cycle
/// of four is short enough for a branch predictor to learn, which would hide the
/// dispatch cost the pipeline is supposed to measure.
fn rotation(i: usize) -> u8 {
    const ORDER: [u8; 8] = [1, 2, 3, 4, 1, 3, 2, 4];
    ORDER[i % ORDER.len()]
}

/// A stream of `PACKETS` tagged packets whose bytes are not compile-time
/// constants. The same linear congruential generator as the other benches, so
/// the payloads are stable between runs and between harnesses.
pub fn stream() -> Vec<u8> {
    let mut out = Vec::with_capacity(PACKETS * 24);
    let mut x: u32 = 0x1234_5678;
    let mut byte = || {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (x >> 24) as u8
    };
    for i in 0..PACKETS {
        let id = rotation(i);
        out.push(id);
        let payload = match id {
            0x01 => 6,
            0x02 => 20,
            0x03 => 10,
            _ => 8,
        };
        for _ in 0..payload {
            out.push(byte());
        }
    }
    out
}

/// One packet of each kind, for the write side.
pub fn packets() -> Vec<Packet> {
    let bytes = stream();
    let mut reader = Reader::new(no_std_io::io::Cursor::new(bytes));
    let mut v = Vec::with_capacity(PACKETS);
    for _ in 0..PACKETS {
        v.push(read_packet(&mut reader));
    }
    v
}
