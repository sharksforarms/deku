use std::io::{Cursor, Read, Seek};

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use deku::prelude::*;

#[cfg(feature = "bits")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuBits {
    #[deku(bits = 1)]
    data_01: u8,
    #[deku(bits = 2)]
    data_02: u8,
    #[deku(bits = 5)]
    data_03: u8,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuBytes {
    data_00: u8,
    data_01: u16,
    data_02: u32,
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
    #[deku(count = "count")]
    data: Vec<u8>,
}

fn deku_write<T: DekuContainerWrite>(input: &T) {
    let _v = input.to_bytes().unwrap();
}

fn deku_read<T: for<'a> DekuContainerRead<'a>>(mut reader: impl Read + Seek) {
    let mut reader = Reader::new(&mut reader);
    let _v = T::from_reader_with_ctx(&mut reader, ()).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("deku_read_byte", |b| {
        let reader = Cursor::new(&[0x01; 1 + 2 + 4]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuBytes>(&mut reader),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("deku_write_byte", |b| {
        b.iter(|| {
            deku_write(black_box(&DekuBytes {
                data_00: 0x00,
                data_01: 0x02,
                data_02: 0x03,
            }))
        })
    });
    #[cfg(feature = "bits")]
    c.bench_function("deku_read_bits", |b| {
        let reader = Cursor::new(&[0x01; 1]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuBits>(&mut reader),
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "bits")]
    c.bench_function("deku_write_bits", |b| {
        b.iter(|| {
            deku_write(black_box(&DekuBits {
                data_01: 0x01,
                data_02: 0x03,
                data_03: 0x06,
            }))
        })
    });

    c.bench_function("deku_read_enum", |b| {
        let reader = Cursor::new(&[0x01; 2]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuEnum>(&mut reader),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("deku_write_enum", |b| {
        b.iter(|| deku_write(black_box(&DekuEnum::VariantA(0x02))))
    });

    let deku_write_vec_input = DekuVec {
        count: 100,
        data: vec![0xff; 100],
    };
    c.bench_function("deku_read_vec", |b| {
        let reader = Cursor::new(&[0x08; 8 + 1]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuVec>(&mut reader),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("deku_write_vec", |b| {
        b.iter(|| deku_write(black_box(&deku_write_vec_input)))
    });
}

pub fn read_all_vs_count(c: &mut Criterion) {
    #[derive(DekuRead, DekuWrite)]
    pub struct AllWrapper {
        #[deku(read_all)]
        pub data: Vec<u8>,
    }

    #[derive(DekuRead, DekuWrite)]
    #[deku(ctx = "len: usize")]
    pub struct CountWrapper {
        #[deku(count = "len")]
        pub data: Vec<u8>,
    }

    c.bench_function("read_all_bytes", |b| {
        b.iter(|| AllWrapper::from_bytes(black_box((&[1; 1500], 0))))
    });

    c.bench_function("read_all", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new([1u8; 1500].as_ref());
            let mut reader = Reader::new(&mut cursor);
            AllWrapper::from_reader_with_ctx(black_box(&mut reader), ())
        })
    });

    c.bench_function("count", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new([1u8; 1500].as_ref());
            let mut reader = Reader::new(&mut cursor);
            CountWrapper::from_reader_with_ctx(black_box(&mut reader), 1500)
        })
    });
}

criterion_group!(benches, criterion_benchmark, read_all_vs_count);
criterion_main!(benches);
