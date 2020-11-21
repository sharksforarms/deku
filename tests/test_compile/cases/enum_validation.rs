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

// test `bits` only allowed on enum
#[derive(DekuRead)]
#[deku(bits = "1")]
struct Test5 {
    a: u8
}

// test `bytes` only allowed on enum
#[derive(DekuRead)]
#[deku(bits = "1")]
struct Test6 {
    a: u8
}

// test `id` only allowed on enum
#[derive(DekuRead)]
#[deku(id = "test")]
struct Test7 {
    a: u8
}

// test `bits` cannot be used with `id`
#[derive(DekuRead)]
#[deku(id = "test", bits = "4")]
enum Test8 {
    A
}

// test `bytes` cannot be used with `id`
#[derive(DekuRead)]
#[deku(id = "test", bytes = "4")]
enum Test9 {
    A
}

// test `id` cannot be `_`
#[derive(DekuRead)]
#[deku(type = "u8")]
enum Test10 {
    #[deku(id = "_")]
    A
}

// test missing `id`
#[derive(DekuRead)]
#[deku(type = "u8")]
enum Test11 {
    #[deku(id = "1")]
    A,
    B(u8)
}

fn main() {}
