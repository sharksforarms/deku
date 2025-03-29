use deku::prelude::*;

// test required attributes
#[derive(DekuRead)]
enum Test1 {}

// test conflict `type` and `id_type`
#[derive(DekuRead)]
#[deku(id_type = "u8", id = "test")]
enum Test2 {}

// test conflict `id_type` and `id_pat`
#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test3 {
    #[deku(id = "1", id_pat = "2..=3")]
    A(u8),
}

// test `id_type` only allowed on enum
#[derive(DekuRead)]
#[deku(id_type = "u8")]
struct Test4 {
    a: u8,
}

// test `bits` only allowed on enum
#[derive(DekuRead)]
#[deku(bits = 1)]
struct Test5 {
    a: u8,
}

// test `bytes` only allowed on enum
#[derive(DekuRead)]
#[deku(bits = 1)]
struct Test6 {
    a: u8,
}

// test `id_type` only allowed on enum
#[derive(DekuRead)]
#[deku(id_type = "test")]
struct Test7 {
    a: u8,
}

// test `bits` cannot be used with `id_type`
#[derive(DekuRead)]
#[deku(id_type = "test", bits = 4)]
enum Test8 {
    A,
}

// test `bytes` cannot be used with `id_type`
#[derive(DekuRead)]
#[deku(id_type = "test", bytes = 4)]
enum Test9 {
    A,
}

// test `type_id` cannot be `_`
#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test10 {
    #[deku(id = "_")]
    A,
}

// test missing `id_type`
#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test11 {
    #[deku(id = "1")]
    A,
    B(u8),
}

// test non matching bits (2) == bits (7)
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8", bits = "2")]
pub enum Test12 {
    #[deku(id_pat = "_")]
    B(#[deku(bits = 7)] u8, #[deku(bits = 6)] u8),
}

// test non matching bytes(2) == bytes (3)
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = "2")]
pub enum Test13 {
    #[deku(id_pat = "_")]
    B(#[deku(bytes = 3)] u32, #[deku(bits = 6)] u8),
}

// test non matching bytes(2) == bytes (3)
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = "2", id_endian = "little")]
pub enum Test14 {
    #[deku(id_pat = "_")]
    B(#[deku(bytes = 2, endian = "big")] u32, #[deku(bits = 6)] u8),
}

// test non matching type
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = "2")]
pub enum Test15 {
    #[deku(id_pat = "_")]
    B(#[deku(bits = 2)] u8, #[deku(bits = 6)] u8),
}

fn main() {}
