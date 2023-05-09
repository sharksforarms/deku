use deku::prelude::*;

#[derive(DekuRead)]
#[deku(type = "u8")]
enum Test1 {
    #[deku(catch_all)]
    A = 1,
    #[deku(catch_all)]
    B = 2,
}

fn main() {}
