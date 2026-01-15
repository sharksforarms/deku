use deku::prelude::*;

#[derive(DekuRead, DekuWrite, DekuSize)]
struct SeekFieldStruct {
    offset: u8,
    #[deku(seek_from_current = "*offset")]
    field: u8,
}

fn main() {
    let _ = SeekFieldStruct::SIZE_BITS;
}
