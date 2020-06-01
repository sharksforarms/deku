use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead)]
struct DekuByte {
    data: u8
}

#[derive(Debug, PartialEq, DekuRead)]
#[deku(id_type = "u8")]
enum DekuEnum {
    #[deku(id = "0x01")]
    VariantA(u8),
}

#[derive(Debug, PartialEq, DekuRead)]
struct DekuVec {
    count: u8,
    #[deku(len = "count")]
    data: Vec<u8>
}

fn deku_read_byte(input: &[u8]) {
    let (_rest, v) = DekuByte::from_bytes((input, 0)).unwrap();
    assert_eq!(DekuByte { data: 0x01 }, v);
}

fn deku_read_enum(input: &[u8]) {
    let (_rest, v) = DekuEnum::from_bytes((input, 0)).unwrap();
    assert_eq!(DekuEnum::VariantA(0x02), v);
}

fn deku_read_vec(input: &[u8]) {
    let (_rest, v) = DekuVec::from_bytes((input, 0)).unwrap();
    assert_eq!(100, v.count);
    assert_eq!(100, v.data.len());
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("deku_read_byte", |b| {
        b.iter(|| deku_read_byte(black_box([0x01].as_ref())))
    });

    c.bench_function("deku_read_enum", |b| {
        b.iter(|| deku_read_enum(black_box([0x01, 0x02].as_ref())))
    });

    let deku_read_vec_input = {
        let mut v = [0xFFu8; 101].to_vec();
        v[0] = 100u8;
        v
    };
    c.bench_function("deku_read_vec", |b| {
        b.iter(|| deku_read_vec(black_box(&deku_read_vec_input)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
