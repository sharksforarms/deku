use deku::prelude::*;

#[derive(DekuSize)]
struct Test {
    data: Vec<u8>,
}

fn main() {
    let _size = Test::SIZE_BITS;
}
