use deku::prelude::*;

#[derive(DekuRead, DekuWrite, DekuSize)]
#[deku(seek_from_start = "10")]
struct SeekStruct {
    field: u8,
}

fn main() {
    let _ = SeekStruct::SIZE_BITS;
}
