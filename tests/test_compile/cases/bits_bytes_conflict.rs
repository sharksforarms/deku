use deku::prelude::*;

#[derive(DekuRead)]
#[deku(id_type = "u8", bits = 1, bytes = 2)]
enum Test1 {}

#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test2 {
    A(#[deku(bits = 1, bytes = 2)] u8),
    B {
        #[deku(bits = 3, bytes = 4)]
        a: u8,
    },
}

#[derive(DekuRead)]
struct Test3 {
    #[deku(bits = 5, bytes = 6)]
    a: u8,
}

#[derive(DekuRead)]
struct Test4(#[deku(bits = 7, bytes = 8)] u8);

fn main() {}
