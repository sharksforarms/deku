use deku::prelude::*;

// test required attributes
#[derive(DekuRead)]
enum Test1 {}

// test conflict `type` and `id`
#[derive(DekuRead)]
#[deku(type = "u8", id = "test")]
enum Test2 {}

// test conflict `id` and `id_pat`
#[derive(DekuRead)]
#[deku(type = "u8")]
enum Test3 {
    #[deku(id = "1", id_pat = "2..=3")] A(u8),
}

// test `type` only allowed on enum
#[derive(DekuRead)]
#[deku(type = "u8")]
struct Test4 {
    a: u8
}

// test `id_bits` only allowed on enum
#[derive(DekuRead)]
#[deku(id_bits = "1")]
struct Test5 {
    a: u8
}

// test `id_bytes` only allowed on enum
#[derive(DekuRead)]
#[deku(id_bits = "1")]
struct Test6 {
    a: u8
}

// test `id` only allowed on enum
#[derive(DekuRead)]
#[deku(id = "test")]
struct Test7 {
    a: u8
}

// test `id_bits` cannot be used with `id`
#[derive(DekuRead)]
#[deku(id = "test", id_bits = "4")]
enum Test8 {
    A
}

// test `id_bytes` cannot be used with `id`
#[derive(DekuRead)]
#[deku(id = "test", id_bytes = "4")]
enum Test9 {
    A
}

fn main() {}
