use deku::prelude::*;

#[derive(DekuRead)]
#[deku(ctx_default = "1")]
struct Test1 {
    a: u8
}


fn main() {}
