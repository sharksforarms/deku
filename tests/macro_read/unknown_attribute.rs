use deku::prelude::*;

#[derive(DekuRead)]
#[deku(a)]
struct Test1 {}

#[derive(DekuRead)]
struct Test2 {
    #[deku(a = 1)]
    a: u8,
}

#[derive(DekuRead)]
#[deku(a(a))]
enum Test3 {}

#[derive(DekuRead)]
enum Test4 {
    #[deku(a = "1")]
    A(),
}

#[derive(DekuRead)]
enum Test5 {
    A(#[deku(a = "1")] u8),
}

#[derive(DekuRead)]
enum Test6 {
    A {
        #[deku(a = "1")]
        a: u8,
    },
}

fn main() {}
