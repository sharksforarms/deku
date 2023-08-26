use std::io::{Cursor, Read};

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
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

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuVec {
    count: u8,
    #[deku(count = "count")]
    data: Vec<u8>,
}

fn deku_read_bits(mut reader: impl Read) {
    let mut container = Container::new(&mut reader);
    let _v = DekuBits::from_reader_with_ctx(&mut container, ()).unwrap();
}

fn deku_write_bits(input: &DekuBits) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_byte(mut reader: impl Read) {
    let mut container = Container::new(&mut reader);
    let _v = DekuBytes::from_reader_with_ctx(&mut container, ()).unwrap();
}

fn deku_write_byte(input: &DekuBytes) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_enum(mut reader: impl Read) {
    let mut container = Container::new(&mut reader);
    let _v = DekuEnum::from_reader_with_ctx(&mut container, ()).unwrap();
}

fn deku_write_enum(input: &DekuEnum) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read_vec(mut reader: impl Read) {
    let mut container = Container::new(&mut reader);
    let _v = DekuVec::from_reader_with_ctx(&mut container, ()).unwrap();
}

fn deku_write_vec(input: &DekuVec) {
    let _v = input.to_bytes().unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("deku_read_byte", |b| {
        let reader = Cursor::new(&[0x01; 1 + 2 + 4]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read_byte(&mut reader),
            BatchSize::SmallInput,
        )
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
        let reader = Cursor::new(&[0x01; 1]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read_bits(&mut reader),
            BatchSize::SmallInput,
        )
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
        let reader = Cursor::new(&[0x01; 2]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read_enum(&mut reader),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("deku_write_enum", |b| {
        b.iter(|| deku_write_enum(black_box(&DekuEnum::VariantA(0x02))))
    });

    let deku_write_vec_input = DekuVec {
        count: 100,
        data: vec![0xff; 100],
    };
    c.bench_function("deku_read_vec", |b| {
        let reader = Cursor::new(&[0x08; 8 + 1]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read_vec(&mut reader),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("deku_write_vec", |b| {
        b.iter(|| deku_write_vec(black_box(&deku_write_vec_input)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
