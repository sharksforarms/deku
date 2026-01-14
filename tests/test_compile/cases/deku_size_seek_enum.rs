use deku::prelude::*;

#[derive(DekuRead, DekuWrite, DekuSize)]
#[deku(id_type = "u8", seek_from_start = "5")]
enum SeekEnum {
    #[deku(id = "1")]
    A,
}

fn main() {
    let _ = SeekEnum::SIZE_BITS;
}
