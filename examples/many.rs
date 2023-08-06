use deku::{bitvec::BitView, container, ctx::Limit, prelude::*, DekuRead, DekuWrite};
use std::io::Write;

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
    let mut container = Container::new(std::io::Cursor::new(custom.clone()));
    let ret = <Vec<Test> as DekuReader<Limit<_, _>>>::from_reader(
        &mut container,
        Limit::new_count(10_0000),
    );

    println!("{:?}", ret);
}
