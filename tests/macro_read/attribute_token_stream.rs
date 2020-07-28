use deku::prelude::*;

#[derive(DekuRead)]
struct Test1 {
    #[deku(cond = "0 == true")]
    a: u8,
}

fn main() {}
