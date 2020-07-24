use deku::prelude::*;

#[derive(DekuRead)]
#[deku(endian = "1")]
struct Test1 {
    a: u8,
}

#[derive(DekuRead)]
struct Test2 {
    #[deku(endian = "2")]
    a: u8,
}

#[derive(DekuRead)]
#[deku(id_type = "u8", endian = "3")]
enum Test3 {}

#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test4 {
    #[deku(id = "1")]
    A(#[deku(endian = "4")] u8),
}

fn main() {}
