use deku::prelude::*;

// Empty skip argument
#[derive(DekuRead, DekuWrite)]
struct TestEmptyArg {
    #[deku(skip())]
    field: u8,
}

fn main() {}
