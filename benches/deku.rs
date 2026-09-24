use no_std_io::io::{Cursor, Read, Seek};

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use deku::prelude::*;

#[path = "shapes/mod.rs"]
mod shapes;
use shapes::{AllWrapper, CountNonSpecialize, CountWrapper, DekuBytes, DekuEnum, DekuVec};
#[cfg(feature = "bits")]
use shapes::{DekuBits, DekuBitsBeU32, DekuBitsBeU64, DekuBitsLeU64};

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
            deku_write(std::hint::black_box(&DekuBytes {
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
            deku_write(std::hint::black_box(&DekuBits {
                data_01: 0x01,
                data_02: 0x03,
                data_03: 0x06,
            }))
        })
    });

    // The endian and width set. `benches/deku_callgrind.rs` counts instructions
    // for the same three reads. Instruction count and time do not move in step
    // on the bit paths, so a difference here needs both harnesses to explain.
    #[cfg(feature = "bits")]
    c.bench_function("deku_read_bits_be_u64", |b| {
        let reader = Cursor::new(&[0xab; 8]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuBitsBeU64>(&mut reader),
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "bits")]
    c.bench_function("deku_read_bits_be_u32", |b| {
        let reader = Cursor::new(&[0xab; 4]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuBitsBeU32>(&mut reader),
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "bits")]
    c.bench_function("deku_read_bits_le_u64", |b| {
        let reader = Cursor::new(&[0xab; 8]);
        b.iter_batched(
            || reader.clone(),
            |mut reader| deku_read::<DekuBitsLeU64>(&mut reader),
            BatchSize::SmallInput,
        )
    });

    #[cfg(feature = "bits")]
    c.bench_function("deku_write_bits_be_u64", |b| {
        b.iter(|| {
            deku_write(std::hint::black_box(&DekuBitsBeU64 {
                data_01: 0x01,
                data_02: 0x03,
                data_03: 0x00ff_ffff_ffff_ffff,
            }))
        })
    });
    #[cfg(feature = "bits")]
    c.bench_function("deku_write_bits_be_u32", |b| {
        b.iter(|| {
            deku_write(std::hint::black_box(&DekuBitsBeU32 {
                data_01: 0x01,
                data_02: 0x7fff_ffff,
            }))
        })
    });
    #[cfg(feature = "bits")]
    c.bench_function("deku_write_bits_le_u64", |b| {
        b.iter(|| {
            deku_write(std::hint::black_box(&DekuBitsLeU64 {
                data_01: 0x01,
                data_02: 0x03,
                data_03: 0x00ff_ffff_ffff_ffff,
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
        b.iter(|| deku_write(std::hint::black_box(&DekuEnum::VariantA(0x02))))
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
        b.iter(|| deku_write(std::hint::black_box(&deku_write_vec_input)))
    });
}

pub fn read_all_vs_count_vs_read_exact(c: &mut Criterion) {
    #[derive(DekuRead, DekuWrite)]
    #[deku(ctx = "len: usize")]
    #[expect(dead_code)]
    pub struct CountFromCtxWrapper {
        #[deku(count = "len")]
        pub data: Vec<u8>,
    }

    c.bench_function("read_all_bytes", |b| {
        b.iter(|| AllWrapper::from_bytes(std::hint::black_box((&[1; 1500], 0))))
    });

    c.bench_function("read_all", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new([1u8; 1500].as_ref());
            let mut reader = Reader::new(&mut cursor);
            AllWrapper::from_reader_with_ctx(std::hint::black_box(&mut reader), ())
        })
    });

    c.bench_function("count_specialize", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new([1u8; 1500].as_ref());
            let mut reader = Reader::new(&mut cursor);
            CountWrapper::from_reader_with_ctx(std::hint::black_box(&mut reader), ())
        })
    });

    c.bench_function("count_from_u8_specialize", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new([1u8; 1500].as_ref());
            let mut reader = Reader::new(&mut cursor);
            CountWrapper::from_reader_with_ctx(std::hint::black_box(&mut reader), ())
        })
    });

    c.bench_function("count_no_specialize", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new([1u8; 1500].as_ref());
            let mut reader = Reader::new(&mut cursor);
            CountNonSpecialize::from_reader_with_ctx(std::hint::black_box(&mut reader), ())
        })
    });
}

criterion_group!(
    benches,
    criterion_benchmark,
    read_all_vs_count_vs_read_exact
);
criterion_main!(benches);
