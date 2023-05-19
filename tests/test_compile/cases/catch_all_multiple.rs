use deku::prelude::*;

#[derive(DekuRead)]
#[deku(type = "u8")]
enum Test1 {
    #[deku(default)]
    A = 1,
    #[deku(default)]
    B = 2,
}

fn main() {}
