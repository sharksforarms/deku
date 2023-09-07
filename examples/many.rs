use deku::{ctx::Limit, prelude::*, DekuRead, DekuWrite};
use std::io::Cursor;

#[derive(Debug, DekuRead, DekuWrite)]
struct Test {
    pub a: u64,
    pub b: u64,
    pub c: u64,
}

fn main() {
    let input: Vec<_> = (0..10_0000)
        .map(|i| Test {
            a: i,
            b: i + 1,
            c: i + 2,
        })
        .collect();
    let custom: Vec<u8> = input
        .iter()
        .flat_map(|x| x.to_bytes().unwrap().into_iter())
        .collect();

    let mut binding = Cursor::new(custom.clone());
    let mut reader = Reader::new(&mut binding);
    let ret = <Vec<Test> as DekuReader<Limit<_, _>>>::from_reader_with_ctx(
        &mut reader,
        Limit::new_count(10_0000),
    );

    println!("{:?}", ret);
}
