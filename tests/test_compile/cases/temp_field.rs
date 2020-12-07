use deku::prelude::*;

// wrong way to use temp
#[derive(DekuRead, DekuWrite)]
struct Test1 {
    #[deku(temp)]
    field_a: u8,
}

fn main() {}
