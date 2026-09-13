use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use no_std_io::io::{Cursor, Read, Seek};

use deku::prelude::*;

#[path = "shapes/mod.rs"]
mod shapes;
use shapes::{
    AllWrapper, CountNonSpecialize, CountWrapper, DekuBits, DekuBitsBeU32, DekuBitsBeU64,
    DekuBitsLeU64, DekuBytes, DekuEnum, DekuVec,
};

fn cursor(bytes: &'static [u8]) -> Cursor<&'static [u8]> {
    Cursor::new(bytes)
}

fn read<T: for<'a> DekuContainerRead<'a>>(mut reader: impl Read + Seek) -> T {
    let mut reader = Reader::new(&mut reader);
    T::from_reader_with_ctx(&mut reader, ()).unwrap()
}

#[library_benchmark]
#[bench::bytes(cursor(&[0x01; 1 + 2 + 4]))]
fn read_bytes(reader: Cursor<&'static [u8]>) -> DekuBytes {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::bits(cursor(&[0x01; 1]))]
fn read_bits(reader: Cursor<&'static [u8]>) -> DekuBits {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::enum_variant(cursor(&[0x01; 2]))]
fn read_enum(reader: Cursor<&'static [u8]>) -> DekuEnum {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::vec(cursor(&[0x08; 8 + 1]))]
fn read_vec(reader: Cursor<&'static [u8]>) -> DekuVec {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::be_u64(cursor(&[0xab; 8]))]
fn read_bits_be_u64(reader: Cursor<&'static [u8]>) -> DekuBitsBeU64 {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::be_u32(cursor(&[0xab; 4]))]
fn read_bits_be_u32(reader: Cursor<&'static [u8]>) -> DekuBitsBeU32 {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::le_u64(cursor(&[0xab; 8]))]
fn read_bits_le_u64(reader: Cursor<&'static [u8]>) -> DekuBitsLeU64 {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::read_all(cursor(&[1u8; 1500]))]
fn read_all(reader: Cursor<&'static [u8]>) -> AllWrapper {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::count_specialize(cursor(&[1u8; 1500]))]
fn read_count_specialize(reader: Cursor<&'static [u8]>) -> CountWrapper {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
#[bench::count_no_specialize(cursor(&[1u8; 1500]))]
fn read_count_no_specialize(reader: Cursor<&'static [u8]>) -> CountNonSpecialize {
    black_box(read(black_box(reader)))
}

#[library_benchmark]
fn write_bytes() -> Vec<u8> {
    black_box(
        black_box(&DekuBytes {
            data_00: 0x00,
            data_01: 0x02,
            data_02: 0x03,
        })
        .to_bytes()
        .unwrap(),
    )
}

#[library_benchmark]
fn write_bits() -> Vec<u8> {
    black_box(
        black_box(&DekuBits {
            data_01: 0x01,
            data_02: 0x03,
            data_03: 0x06,
        })
        .to_bytes()
        .unwrap(),
    )
}

#[library_benchmark]
fn write_enum() -> Vec<u8> {
    black_box(black_box(&DekuEnum::VariantA(0x02)).to_bytes().unwrap())
}

#[library_benchmark]
fn write_vec() -> Vec<u8> {
    let input = DekuVec {
        count: 100,
        data: vec![0xff; 100],
    };
    black_box(black_box(&input).to_bytes().unwrap())
}

library_benchmark_group!(
    name = read_group,
    benchmarks = [
        read_bytes,
        read_bits,
        read_enum,
        read_vec,
        read_bits_be_u64,
        read_bits_be_u32,
        read_bits_le_u64,
        read_all,
        read_count_specialize,
        read_count_no_specialize,
    ]
);

library_benchmark_group!(
    name = write_group,
    benchmarks = [write_bytes, write_bits, write_enum, write_vec]
);

main!(library_benchmark_groups = read_group, write_group);
