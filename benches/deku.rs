use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deku::prelude::*;

#[derive(DekuRead)]
struct DekuTest {
    f1: u8,
    f2: u8,
    f3: u8,
}

fn deku_read(input: &[u8]) {
    DekuTest::from_bytes((input, 0));
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("deku_read", |b| {
        b.iter(|| deku_read(black_box([0x01, 0x02, 0x03].as_ref())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
