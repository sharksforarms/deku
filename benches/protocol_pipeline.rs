//! Wall time for a multi-protocol pipeline. `benches/protocol_pipeline_callgrind.rs`
//! counts instructions for the same shapes.
//!
//! This is the case the comment above the read loop in `Reader::read_bits_uint_msb0`
//! says a wide `read_exact` regresses: narrow fields, several shapes, and a
//! dispatch the branch predictor cannot learn. The single-shape benches in
//! `benches/bebits.rs` cannot show it.
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use deku::prelude::*;
use no_std_io::io::Cursor;

#[path = "protocol/mod.rs"]
mod protocol;
use protocol::{packets, read_packet, stream, BbHeader, Ipv4Header, Packet, SpacePacket, PACKETS};

fn bench(c: &mut Criterion) {
    let bytes = stream();

    // The pipeline: dispatch on the id byte, decode whichever shape arrives.
    c.bench_function("pipeline_mixed_x256", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&bytes)));
            let mut acc: u64 = 0;
            for _ in 0..PACKETS {
                acc ^= read_packet(&mut r).fold();
            }
            acc
        })
    });

    // Each shape on its own, for attribution: if the pipeline moves, these say
    // which shape moved.
    let space: Vec<u8> = {
        let mut v = Vec::new();
        let all = stream();
        let mut r = Reader::new(Cursor::new(&all));
        for _ in 0..PACKETS {
            let p = read_packet(&mut r);
            if let Packet::Space(s) = p {
                v.extend_from_slice(&s.to_bytes().unwrap());
            }
        }
        v
    };
    let n_space = space.len() / 6;
    c.bench_function("pipeline_space_only", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&space)));
            let mut acc: u64 = 0;
            for _ in 0..n_space {
                let s = SpacePacket::from_reader_with_ctx(&mut r, ()).unwrap();
                acc ^= u64::from(s.apid) ^ u64::from(s.seq_count);
            }
            acc
        })
    });

    let ipv4: Vec<u8> = {
        let mut v = Vec::new();
        let all = stream();
        let mut r = Reader::new(Cursor::new(&all));
        for _ in 0..PACKETS {
            if let Packet::Ipv4(p) = read_packet(&mut r) {
                v.extend_from_slice(&p.to_bytes().unwrap());
            }
        }
        v
    };
    let n_ipv4 = ipv4.len() / 20;
    c.bench_function("pipeline_ipv4_only", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&ipv4)));
            let mut acc: u64 = 0;
            for _ in 0..n_ipv4 {
                let p = Ipv4Header::from_reader_with_ctx(&mut r, ()).unwrap();
                acc ^= u64::from(p.src) ^ u64::from(p.total_length);
            }
            acc
        })
    });

    let bb: Vec<u8> = {
        let mut v = Vec::new();
        let all = stream();
        let mut r = Reader::new(Cursor::new(&all));
        for _ in 0..PACKETS {
            if let Packet::Bb(p) = read_packet(&mut r) {
                v.extend_from_slice(&p.to_bytes().unwrap());
            }
        }
        v
    };
    let n_bb = bb.len() / 10;
    c.bench_function("pipeline_bb_only", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(black_box(&bb)));
            let mut acc: u64 = 0;
            for _ in 0..n_bb {
                let p = BbHeader::from_reader_with_ctx(&mut r, ()).unwrap();
                acc ^= u64::from(p.dfl) ^ u64::from(p.upl);
            }
            acc
        })
    });

    // Write side of the pipeline.
    let pkts = packets();
    c.bench_function("pipeline_write_mixed_x256", |b| {
        let mut out = vec![0u8; bytes.len() + 64];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            for p in black_box(&pkts) {
                p.to_writer(&mut w, ()).unwrap();
            }
            w.finalize().unwrap();
        })
    });
}
criterion_group!(protocol_pipeline, bench);
criterion_main!(protocol_pipeline);
