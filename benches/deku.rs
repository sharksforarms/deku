use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuByte {
    data: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuEnum {
    #[deku(id = "0x01")]
    VariantA(u8),
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuVec {
    count: u8,
    #[deku(len = "count")]
    data: Vec<u8>,
}

fn deku_read_byte(input: &[u8]) {
    let (_rest, _v) = DekuByte::from_bytes((input, 0)).unwrap();
}

fn deku_write_byte(input: &DekuByte) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_enum(input: &[u8]) {
    let (_rest, _v) = DekuEnum::from_bytes((input, 0)).unwrap();
}

fn deku_write_enum(input: &DekuEnum) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_vec(input: &[u8]) {
    let (_rest, _v) = DekuVec::from_bytes((input, 0)).unwrap();
}

fn deku_write_vec(input: &DekuVec) {
    let _v = input.to_bytes().unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("deku_read_byte", |b| {
        b.iter(|| deku_read_byte(black_box([0x01].as_ref())))
    });
    c.bench_function("deku_write_byte", |b| {
        b.iter(|| deku_write_byte(black_box(&DekuByte { data: 0x01 })))
    });

    c.bench_function("deku_read_enum", |b| {
        b.iter(|| deku_read_enum(black_box([0x01, 0x02].as_ref())))
    });
    c.bench_function("deku_write_enum", |b| {
        b.iter(|| deku_write_enum(black_box(&DekuEnum::VariantA(0x02))))
    });

    let deku_read_vec_input = {
        let mut v = [0xFFu8; 101].to_vec();
        v[0] = 100u8;
        v
    };
    let deku_write_vec_input = DekuVec {
        count: 100,
        data: vec![0xFF; 100],
    };
    c.bench_function("deku_read_vec", |b| {
        b.iter(|| deku_read_vec(black_box(&deku_read_vec_input)))
    });
    c.bench_function("deku_write_vec", |b| {
        b.iter(|| deku_write_vec(black_box(&deku_write_vec_input)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
