//! Big-endian bit-packed headers: the shape used by real network / space
//! protocols (CCSDS, IPv4, DVB-S2). Mirrors the CCSDS TM Transfer Frame
//! primary header, 6 octets / 11 fields.
//!
//! Each shape is measured twice.
//!
//! The single-shot benches handle one struct per iteration. Consecutive
//! iterations share no state, so the CPU overlaps them and the result is a
//! throughput figure rather than the cost of one read.
//!
//! The `_xN` benches move N structs through one reader or writer, which is what
//! a stream of frames actually does: each read depends on where the previous one
//! left the cursor, so nothing overlaps. Every field is folded into a value the
//! closure returns, so no read can be dropped. Divide by N for the per-struct
//! cost; that is the number to quote.
use criterion::{criterion_group, criterion_main, Criterion};
use deku::prelude::*;
use no_std_io::io::Cursor;
use std::hint::black_box;

/// Frames per sequential pass. 128 six-octet frames is 768 bytes, comfortably
/// inside L1 so the measurement is field decoding rather than memory.
const FRAMES: usize = 128;

/// 1-bit fields per sequential pass: 1024 bits, i.e. 128 bytes.
const BITS: usize = 1024;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct TmPrimaryHeader {
    #[deku(bits = 2)]
    tfvn: u8,
    #[deku(bits = 10)]
    scid: u16,
    #[deku(bits = 3)]
    vcid: u8,
    #[deku(bits = 1)]
    ocf: u8,
    mcfc: u8,
    vcfc: u8,
    #[deku(bits = 1)]
    tfs: u8,
    #[deku(bits = 1)]
    syn: u8,
    #[deku(bits = 1)]
    po: u8,
    #[deku(bits = 2)]
    sli: u8,
    #[deku(bits = 11)]
    fhp: u16,
}

/// Same 6 octets, byte-aligned: the deku fast path, for scale. Also the control
/// for any change to the bit paths, which must leave this one alone.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct SixBytes {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
}

/// Worst case in the docs: a 1-bit field in a wide container.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct OneBitU64 {
    #[deku(bits = 1)]
    a: u64,
}

/// A frame stream whose bytes are not compile-time constants.
fn stream() -> [u8; FRAMES * 6] {
    let mut buf = [0u8; FRAMES * 6];
    let mut x: u32 = 0x1234_5678;
    for b in buf.iter_mut() {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *b = (x >> 24) as u8;
    }
    buf
}

fn bench(c: &mut Criterion) {
    let buf = [0x2Au8, 0xB5, 0x11, 0x22, 0xC7, 0xFF, 0x00, 0x99];
    let stream = stream();

    // One struct per iteration.
    c.bench_function("be_tm_primary_header_11_fields", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&buf)));
            TmPrimaryHeader::from_reader_with_ctx(&mut r, ()).unwrap()
        })
    });
    c.bench_function("be_six_bytes_aligned", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&buf)));
            SixBytes::from_reader_with_ctx(&mut r, ()).unwrap()
        })
    });
    c.bench_function("be_one_bit_in_u64", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&buf)));
            OneBitU64::from_reader_with_ctx(&mut r, ()).unwrap()
        })
    });

    // A stream of frames through one reader.
    c.bench_function("be_tm_primary_header_x128", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&stream)));
            let mut acc: u64 = 0;
            for _ in 0..FRAMES {
                let h = TmPrimaryHeader::from_reader_with_ctx(&mut r, ()).unwrap();
                acc ^= u64::from(h.scid)
                    ^ u64::from(h.fhp)
                    ^ u64::from(h.mcfc)
                    ^ u64::from(h.vcid)
                    ^ u64::from(h.tfvn)
                    ^ u64::from(h.sli);
            }
            acc
        })
    });
    c.bench_function("be_six_bytes_aligned_x128", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&stream)));
            let mut acc: u64 = 0;
            for _ in 0..FRAMES {
                let s = SixBytes::from_reader_with_ctx(&mut r, ()).unwrap();
                acc ^= u64::from(s.a) ^ u64::from(s.f);
            }
            acc
        })
    });
    c.bench_function("be_one_bit_in_u64_x1024", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&stream)));
            let mut acc: u64 = 0;
            for _ in 0..BITS {
                acc ^= OneBitU64::from_reader_with_ctx(&mut r, ()).unwrap().a;
            }
            acc
        })
    });

    // Write side of the same shapes. Into a reused stack buffer, so the
    // measurement is the field writes rather than an allocation.
    let mut r = Reader::new(Cursor::new(&buf));
    let header = TmPrimaryHeader::from_reader_with_ctx(&mut r, ()).unwrap();
    let six = SixBytes {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: 5,
        f: 6,
    };
    c.bench_function("be_write_tm_primary_header_11_fields", |b| {
        let mut out = [0u8; 16];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            black_box(&header).to_writer(&mut w, ()).unwrap();
            w.finalize().unwrap();
        })
    });
    c.bench_function("be_write_six_bytes_aligned", |b| {
        let mut out = [0u8; 16];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            black_box(&six).to_writer(&mut w, ()).unwrap();
            w.finalize().unwrap();
        })
    });

    // A stream of frames through one writer.
    c.bench_function("be_write_tm_primary_header_x128", |b| {
        let mut out = [0u8; FRAMES * 6];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            for _ in 0..FRAMES {
                black_box(&header).to_writer(&mut w, ()).unwrap();
            }
            w.finalize().unwrap();
        })
    });
    c.bench_function("be_write_six_bytes_aligned_x128", |b| {
        let mut out = [0u8; FRAMES * 6];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            for _ in 0..FRAMES {
                black_box(&six).to_writer(&mut w, ()).unwrap();
            }
            w.finalize().unwrap();
        })
    });
}
criterion_group!(bebits, bench);
criterion_main!(bebits);
