//! Wall time for the big-endian bit-packed shapes in [`common`].
//! `benches/bebits_callgrind.rs` measures the same shapes for instruction
//! counts.
//!
//! This harness measures real silicon, so it sees cache behaviour, branch
//! prediction, and instruction-level parallelism that callgrind cannot. It is
//! also noisy on a shared machine. Read the two harnesses together: instruction
//! count and time do not move in step, and on these shapes they disagree by up
//! to two orders of magnitude.
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
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use deku::prelude::*;
use no_std_io::io::Cursor;

#[path = "common/mod.rs"]
mod common;
use common::{
    addresses, header, six, stream_array, Addresses, OneBitU64, SixBytes, TmPrimaryHeader, ADDRS,
    BITS, FRAMES, HEADER_BYTES,
};

fn bench(c: &mut Criterion) {
    let buf = HEADER_BYTES;
    let stream = stream_array();

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
    c.bench_function("be_byte_arrays_x96", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&stream)));
            let mut acc: u64 = 0;
            for _ in 0..ADDRS {
                let a = Addresses::from_reader_with_ctx(&mut r, ()).unwrap();
                acc ^= u64::from(a.source[0]) ^ u64::from(a.destination[3]);
            }
            acc
        })
    });

    // Write side of the same shapes. Into a reused stack buffer, so the
    // measurement is the field writes rather than an allocation.
    let header = header();
    let six = six();
    let addresses = addresses();
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

    c.bench_function("be_write_byte_arrays_x96", |b| {
        let mut out = [0u8; ADDRS * 8];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            for _ in 0..ADDRS {
                black_box(&addresses).to_writer(&mut w, ()).unwrap();
            }
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
