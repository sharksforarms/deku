use std::io::Read;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deku::container::Container;
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuBits {
    #[deku(bits = "1")]
    data_01: u8,
    #[deku(bits = "7")]
    data_02: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuBytes {
    data_00: u8,
    data_01: u16,
    data_02: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
enum DekuEnum {
    #[deku(id = "0x01")]
    VariantA(u8),
}

/// This is faster, because we go right to (endian, bytes)
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuVecPerf {
    #[deku(bytes = "1")]
    count: u8,
    #[deku(count = "count")]
    #[deku(bytes = "1")]
    data: Vec<u8>,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuVec {
    count: u8,
    #[deku(count = "count")]
    data: Vec<u8>,
}

fn deku_read_bits(reader: impl Read) {
    let mut container = Container::new(reader);
    let _v = DekuBits::from_reader(&mut container, ()).unwrap();
}

fn deku_write_bits(input: &DekuBits) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_byte(reader: impl Read) {
    let mut container = Container::new(reader);
    let _v = DekuBytes::from_reader(&mut container, ()).unwrap();
}

fn deku_write_byte(input: &DekuBytes) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_enum(input: &[u8]) {
    let mut container = Container::new(input);
    let _v = DekuEnum::from_reader(&mut container, ()).unwrap();
}

fn deku_write_enum(input: &DekuEnum) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_vec(input: &[u8]) {
    let mut container = Container::new(input);
    let _v = DekuVec::from_reader(&mut container, ()).unwrap();
}

fn deku_write_vec(input: &DekuVec) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_vec_perf(input: &[u8]) {
    let mut container = Container::new(std::io::Cursor::new(input));
    let _v = DekuVecPerf::from_reader(&mut container, ()).unwrap();
}

fn deku_write_vec_perf(input: &DekuVecPerf) {
    let _v = input.to_bytes().unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut reader = std::io::repeat(0b101);
    c.bench_function("deku_read_byte", |b| {
        b.iter(|| deku_read_byte(black_box(&mut reader)))
    });
    c.bench_function("deku_write_byte", |b| {
        b.iter(|| {
            deku_write_byte(black_box(&DekuBytes {
                data_00: 0x00,
                data_01: 0x02,
                data_02: 0x03,
            }))
        })
    });
    c.bench_function("deku_read_bits", |b| {
        b.iter(|| deku_read_bits(black_box(&mut reader)))
    });
    c.bench_function("deku_write_bits", |b| {
        b.iter(|| {
            deku_write_bits(black_box(&DekuBits {
                data_01: 0x0f,
                data_02: 0x01,
            }))
        })
    });

    c.bench_function("deku_read_enum", |b| {
        b.iter(|| deku_read_enum(black_box([0x01, 0x02].as_ref())))
    });
    c.bench_function("deku_write_enum", |b| {
        b.iter(|| deku_write_enum(black_box(&DekuEnum::VariantA(0x02))))
    });

    let deku_read_vec_input = {
        let mut v = [0xffu8; 101].to_vec();
        v[0] = 100u8;
        v
    };
    let deku_write_vec_input = DekuVec {
        count: 100,
        data: vec![0xff; 100],
    };
    c.bench_function("deku_read_vec", |b| {
        b.iter(|| deku_read_vec(black_box(&deku_read_vec_input)))
    });
    c.bench_function("deku_write_vec", |b| {
        b.iter(|| deku_write_vec(black_box(&deku_write_vec_input)))
    });

    let deku_write_vec_input = DekuVecPerf {
        count: 100,
        data: vec![0xff; 100],
    };
    c.bench_function("deku_read_vec_perf", |b| {
        b.iter(|| deku_read_vec_perf(black_box(&deku_read_vec_input)))
    });
    c.bench_function("deku_write_vec_perf", |b| {
        b.iter(|| deku_write_vec_perf(black_box(&deku_write_vec_input)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
