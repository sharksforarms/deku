use deku::prelude::*;

// Invalid skip argument
#[derive(DekuRead, DekuWrite)]
struct TestInvalidArg {
    #[deku(skip(invalid))]
    field: u8,
}

fn main() {}
