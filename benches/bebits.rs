//! Big-endian bit-packed headers: the shape used by real network / space
//! protocols (CCSDS, IPv4, DVB-S2). Mirrors the CCSDS TM Transfer Frame
//! primary header, 6 octets / 11 fields.
use criterion::{criterion_group, criterion_main, Criterion};
use deku::prelude::*;
use no_std_io::io::Cursor;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct TmPrimaryHeader {
    #[deku(bits = 2)]
    tfvn: u8,
    #[deku(bits = 10)]
    scid: u16,
    #[deku(bits = 3)]
    vcid: u8,
    #[deku(bits = 1)]
    ocf: u8,
    mcfc: u8,
    vcfc: u8,
    #[deku(bits = 1)]
    tfs: u8,
    #[deku(bits = 1)]
    syn: u8,
    #[deku(bits = 1)]
    po: u8,
    #[deku(bits = 2)]
    sli: u8,
    #[deku(bits = 11)]
    fhp: u16,
}

/// Same 6 octets, byte-aligned: the deku fast path, for scale.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct SixBytes {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
}

/// Worst case in the docs: a 1-bit field in a wide container.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct OneBitU64 {
    #[deku(bits = 1)]
    a: u64,
}

fn bench(c: &mut Criterion) {
    let buf = [0x2Au8, 0xB5, 0x11, 0x22, 0xC7, 0xFF, 0x00, 0x99];
    c.bench_function("be_tm_primary_header_11_fields", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(&buf));
            TmPrimaryHeader::from_reader_with_ctx(&mut r, ()).unwrap()
        })
    });
    c.bench_function("be_six_bytes_aligned", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(&buf));
            SixBytes::from_reader_with_ctx(&mut r, ()).unwrap()
        })
    });
    c.bench_function("be_one_bit_in_u64", |b| {
        b.iter(|| {
            let mut r = Reader::new(Cursor::new(&buf));
            OneBitU64::from_reader_with_ctx(&mut r, ()).unwrap()
        })
    });

    // Write side of the same shapes. Into a reused stack buffer, so the
    // measurement is the field writes rather than an allocation.
    let mut r = Reader::new(Cursor::new(&buf));
    let header = TmPrimaryHeader::from_reader_with_ctx(&mut r, ()).unwrap();
    let six = SixBytes {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: 5,
        f: 6,
    };
    c.bench_function("be_write_tm_primary_header_11_fields", |b| {
        let mut out = [0u8; 16];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            header.to_writer(&mut w, ()).unwrap();
            w.finalize().unwrap();
        })
    });
    c.bench_function("be_write_six_bytes_aligned", |b| {
        let mut out = [0u8; 16];
        b.iter(|| {
            let mut w = Writer::new(Cursor::new(out.as_mut_slice()));
            six.to_writer(&mut w, ()).unwrap();
            w.finalize().unwrap();
        })
    });
}
criterion_group!(bebits, bench);
criterion_main!(bebits);
