use deku::prelude::*;

#[derive(DekuRead)]
#[deku(endian = "variable")]
struct Test1 {
    a: u8,
}

#[derive(DekuRead)]
struct Test2 {
    #[deku(endian = "variable")]
    a: u8,
}

#[derive(DekuRead)]
#[deku(type = "u8", endian = "variable")]
enum Test3 {}

#[derive(DekuRead)]
#[deku(type = "u8")]
enum Test4 {
    #[deku(id = "1")]
    A(#[deku(endian = "variable")] u8),
}

fn main() {}
