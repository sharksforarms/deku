//! Instruction counts for the multi-protocol pipeline. `benches/protocol_pipeline.rs`
//! measures wall time for the same shapes.
//!
//! Callgrind cannot see branch misprediction, which is part of what a dispatch
//! loop costs. Read this next to the criterion figures, not instead of them.
use std::hint::black_box;

use deku::prelude::*;
use gungraun::{library_benchmark, library_benchmark_group, main};
use no_std_io::io::Cursor;

#[path = "protocol/mod.rs"]
mod protocol;
use protocol::{packets, read_packet, stream, Packet, PACKETS};

#[library_benchmark]
#[bench::mixed(Cursor::new(stream()))]
fn pipeline_mixed_x256(stream: Cursor<Vec<u8>>) -> u64 {
    let mut r = Reader::new(black_box(stream));
    let mut acc: u64 = 0;
    for _ in 0..PACKETS {
        acc ^= read_packet(&mut r).fold();
    }
    black_box(acc)
}

#[library_benchmark]
#[bench::mixed(packets())]
fn pipeline_write_mixed_x256(pkts: Vec<Packet>) {
    let mut out = vec![0u8; 256 * 24 + 64];
    let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
    for p in black_box(&pkts) {
        p.to_writer(&mut w, ()).unwrap();
    }
    w.finalize().unwrap();
    black_box(&out);
}

library_benchmark_group!(name = read_group, benchmarks = [pipeline_mixed_x256]);
library_benchmark_group!(name = write_group, benchmarks = [pipeline_write_mixed_x256]);
main!(library_benchmark_groups = read_group, write_group);
