use deku::prelude::*;

// With no `endian`, the field is in the target's order, which no run reads.
#[derive(DekuBitField)]
struct NotBigEndian {
    #[deku(bits = 4)]
    a: u8,
}

#[derive(DekuBitField)]
#[deku(endian = "big")]
struct NotARunField {
    #[deku(count = "2")]
    a: Vec<u8>,
}

#[derive(DekuBitField)]
#[deku(id_type = "u8", bits = 2, endian = "big")]
enum VariantsOfTwoWidths {
    #[deku(id = 0)]
    A(#[deku(bits = 3)] u8),
    #[deku(id = 1)]
    B(#[deku(bits = 4)] u8),
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
