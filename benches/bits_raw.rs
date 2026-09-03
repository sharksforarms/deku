use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use deku::bitvec::{BitSlice, BitSliceMut, BitVec, Msb0};
use deku::ctx::Order;
use deku::prelude::Reader;
use no_std_io::io::Cursor;
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let source_bytes = [0xA5u8; 32];
    let source: BitSlice<'_, u8, Msb0> = BitSlice::from_slice(&source_bytes);
    let aligned_source_bytes = black_box([
        0xA5, 0x3C, 0x7E, 0x19, 0xD2, 0x48, 0xF0, 0x06, 0x91, 0xB4, 0x2D, 0xC8, 0x57, 0xE3, 0x10,
        0x6B, 0x84, 0x3A, 0xF7, 0x25, 0xCC, 0x08, 0xD9, 0x61, 0xAE, 0x42, 0x17, 0xFB, 0x70, 0x34,
        0xC1, 0x5E,
    ]);
    let aligned_source: BitSlice<'_, u8, Msb0> =
        BitSlice::from_slice(black_box(&aligned_source_bytes));

    c.bench_function("bits_load_be_8", |b| {
        b.iter(|| black_box(source.subslice(3, 11).load_be::<u8>()))
    });
    c.bench_function("bits_load_be_64", |b| {
        b.iter(|| black_box(source.subslice(3, 67).load_be::<u64>()))
    });
    c.bench_function("bits_load_be_aligned_64", |b| {
        b.iter(|| black_box(aligned_source.subslice(0, 64).load_be::<u64>()))
    });
    c.bench_function("bits_load_le_aligned_64", |b| {
        b.iter(|| black_box(aligned_source.subslice(0, 64).load_le::<u64>()))
    });
    c.bench_function("bits_load_le_unaligned_64", |b| {
        b.iter(|| black_box(source.subslice(3, 67).load_le::<u64>()))
    });
    c.bench_function("bits_copy_aligned_128", |b| {
        b.iter(|| {
            let mut destination_bytes = [0u8; 16];
            let mut destination: BitSliceMut<'_, u8, Msb0> =
                BitSliceMut::from_slice(&mut destination_bytes);
            destination.copy_from_bitslice(&source.subslice(0, 128));
            black_box(destination_bytes)
        })
    });
    c.bench_function("bits_copy_unaligned_128", |b| {
        b.iter(|| {
            let mut destination_bytes = [0u8; 18];
            let mut destination: BitSliceMut<'_, u8, Msb0> =
                BitSliceMut::from_slice(&mut destination_bytes);
            let mut destination = destination.subslice(1, 129);
            destination.copy_from_bitslice(&source.subslice(3, 131));
            black_box(destination_bytes)
        })
    });
    let source_bits = [true; 128];
    c.bench_function("bits_vec_from_bits_128", |b| {
        b.iter(|| black_box(BitVec::<u8>::from_bits(black_box(&source_bits))))
    });
    c.bench_function("bits_reader_read_bits_128", |b| {
        b.iter_batched(
            || Reader::new(Cursor::new(source_bytes)),
            |mut reader| black_box(reader.read_bits(128, Order::Msb0).unwrap()),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("bits_reader_read_bits_127", |b| {
        b.iter_batched(
            || Reader::new(Cursor::new(source_bytes)),
            |mut reader| black_box(reader.read_bits(127, Order::Msb0).unwrap()),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("bits_reader_read_bits_lsb0_128", |b| {
        b.iter_batched(
            || Reader::new(Cursor::new(source_bytes)),
            |mut reader| black_box(reader.read_bits(128, Order::Lsb0).unwrap()),
            BatchSize::SmallInput,
        )
    });
    c.bench_function("bits_reader_read_bits_lsb0_127", |b| {
        b.iter_batched(
            || Reader::new(Cursor::new(source_bytes)),
            |mut reader| black_box(reader.read_bits(127, Order::Lsb0).unwrap()),
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(bits_raw, bench);
criterion_main!(bits_raw);
