//! Instruction counts for the big-endian bit-packed shapes in [`common`].
//! `benches/bebits.rs` measures the same shapes for wall time.
//!
//! Callgrind counts instructions on a simulated CPU. The count does not change
//! between runs, which makes it the harness to gate a pull request on, but it
//! cannot see cache behaviour, branch prediction, or instruction-level
//! parallelism. Read the two harnesses together: a change that removes
//! instructions but makes the remaining ones serialize is not an improvement.
//!
//! Each shape is measured twice.
//!
//! The single-shot benches handle one struct per iteration. Consecutive
//! iterations share no state, so the CPU overlaps them and the result is a
//! throughput figure rather than the cost of one read.
//!
//! The `_xN` benches move N structs through one reader or writer, which is what
//! a stream of frames actually does: each read depends on where the previous one
//! left the cursor, so nothing overlaps. Reads fold every field into a returned
//! value, and writes keep the filled buffer alive with `black_box`. Thus the
//! compiler cannot remove the work. Divide by N for the per-struct cost; that is
//! the number to quote.
use std::hint::black_box;

use deku::prelude::*;
use gungraun::{library_benchmark, library_benchmark_group, main};
use no_std_io::io::Cursor;

#[path = "common/mod.rs"]
mod common;
use common::{
    addresses, header, read, six, stream_vec, Addresses, OneBitU64, SixBytes, TmPrimaryHeader,
    ADDRS, BITS, FRAMES, HEADER_BYTES,
};

fn cursor() -> Cursor<&'static [u8]> {
    Cursor::new(&HEADER_BYTES[..])
}

#[library_benchmark]
#[bench::bytes(cursor())]
fn be_tm_primary_header_11_fields(reader: Cursor<&'static [u8]>) -> TmPrimaryHeader {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::bytes(cursor())]
fn be_six_bytes_aligned(reader: Cursor<&'static [u8]>) -> SixBytes {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::bytes(cursor())]
fn be_one_bit_in_u64(reader: Cursor<&'static [u8]>) -> OneBitU64 {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::bytes(Cursor::new(stream_vec()))]
fn be_tm_primary_header_x128(stream: Cursor<Vec<u8>>) -> u64 {
    let mut r = Reader::new(black_box(stream));
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
    black_box(acc)
}

#[library_benchmark]
#[bench::bytes(Cursor::new(stream_vec()))]
fn be_six_bytes_aligned_x128(stream: Cursor<Vec<u8>>) -> u64 {
    let mut r = Reader::new(black_box(stream));
    let mut acc: u64 = 0;
    for _ in 0..FRAMES {
        let s = SixBytes::from_reader_with_ctx(&mut r, ()).unwrap();
        acc ^= u64::from(s.a) ^ u64::from(s.f);
    }
    black_box(acc)
}

#[library_benchmark]
#[bench::bytes(Cursor::new(stream_vec()))]
fn be_one_bit_in_u64_x1024(stream: Cursor<Vec<u8>>) -> u64 {
    let mut r = Reader::new(black_box(stream));
    let mut acc: u64 = 0;
    for _ in 0..BITS {
        acc ^= OneBitU64::from_reader_with_ctx(&mut r, ()).unwrap().a;
    }
    black_box(acc)
}

#[library_benchmark]
#[bench::bytes(Cursor::new(stream_vec()))]
fn be_byte_arrays_x96(stream: Cursor<Vec<u8>>) -> u64 {
    let mut r = Reader::new(black_box(stream));
    let mut acc: u64 = 0;
    for _ in 0..ADDRS {
        let a = Addresses::from_reader_with_ctx(&mut r, ()).unwrap();
        acc ^= u64::from(a.source[0]) ^ u64::from(a.destination[3]);
    }
    black_box(acc)
}

// Write side of the same shapes. Into a reused stack buffer, so the
// measurement is the field writes rather than an allocation.

#[library_benchmark]
#[bench::header(header())]
fn be_write_tm_primary_header_11_fields(header: TmPrimaryHeader) {
    let mut out = [0u8; 16];
    let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
    black_box(&header).to_writer(&mut w, ()).unwrap();
    w.finalize().unwrap();
    black_box(&out);
}

#[library_benchmark]
#[bench::six(six())]
fn be_write_six_bytes_aligned(six: SixBytes) {
    let mut out = [0u8; 16];
    let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
    black_box(&six).to_writer(&mut w, ()).unwrap();
    w.finalize().unwrap();
    black_box(&out);
}

#[library_benchmark]
#[bench::addresses(addresses())]
fn be_write_byte_arrays_x96(addresses: Addresses) {
    let mut out = [0u8; ADDRS * 8];
    let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
    for _ in 0..ADDRS {
        black_box(&addresses).to_writer(&mut w, ()).unwrap();
    }
    w.finalize().unwrap();
    black_box(&out);
}

#[library_benchmark]
#[bench::header(header())]
fn be_write_tm_primary_header_x128(header: TmPrimaryHeader) {
    let mut out = [0u8; FRAMES * 6];
    let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
    for _ in 0..FRAMES {
        black_box(&header).to_writer(&mut w, ()).unwrap();
    }
    w.finalize().unwrap();
    black_box(&out);
}

#[library_benchmark]
#[bench::six(six())]
fn be_write_six_bytes_aligned_x128(six: SixBytes) {
    let mut out = [0u8; FRAMES * 6];
    let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
    for _ in 0..FRAMES {
        black_box(&six).to_writer(&mut w, ()).unwrap();
    }
    w.finalize().unwrap();
    black_box(&out);
}

library_benchmark_group!(
    name = read_group,
    benchmarks = [
        be_tm_primary_header_11_fields,
        be_six_bytes_aligned,
        be_one_bit_in_u64,
        be_tm_primary_header_x128,
        be_six_bytes_aligned_x128,
        be_one_bit_in_u64_x1024,
        be_byte_arrays_x96,
    ]
);

library_benchmark_group!(
    name = write_group,
    benchmarks = [
        be_write_tm_primary_header_11_fields,
        be_write_six_bytes_aligned,
        be_write_tm_primary_header_x128,
        be_write_six_bytes_aligned_x128,
        be_write_byte_arrays_x96,
    ]
);
main!(library_benchmark_groups = read_group, write_group);
