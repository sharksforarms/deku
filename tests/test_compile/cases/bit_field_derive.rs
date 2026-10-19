use deku::prelude::*;

#[derive(DekuBitField)]
struct NotAnEnum {
    a: u8,
}

#[derive(DekuBitField)]
#[deku(id_type = "u8", bits = 2)]
enum HasDefault {
    #[deku(id = 0)]
    A,
    #[deku(id_pat = "_")]
    Other,
}

#[derive(DekuBitField)]
#[deku(id_type = "u8", bits = 2)]
enum IdTooWide {
    #[deku(id = 0b01)]
    A,
    #[deku(id = 0b101)]
    B,
}

#[derive(DekuBitField)]
#[deku(id_type = "u16", bits = 12, endian = "little")]
enum LittleEndian {
    #[deku(id = 1)]
    A,
}

#[derive(DekuBitField)]
#[deku(id_type = "i8")]
enum Signed {
    #[deku(id = 1)]
    A,
}

fn main() {}
