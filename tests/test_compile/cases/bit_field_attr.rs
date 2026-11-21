use deku::prelude::*;

#[derive(DekuRead, DekuWrite, DekuBitField)]
#[deku(id_type = "u8", bits = 1, ctx = "k: u8")]
enum NeedsCtx {
    #[deku(id = 0)]
    A,
    #[deku(id = 1)]
    B,
}

#[derive(DekuRead)]
#[deku(endian = "big")]
struct BitsConflict {
    #[deku(bits = 7)]
    a: u8,
    #[deku(bit_field, bits = 1)]
    b: NeedsCtx,
}

// A batched run still type-checks each field's own read, so a type expecting a
// context the struct does not pass is rejected as it is without `bit_field`.
#[derive(DekuRead)]
#[deku(endian = "big")]
struct MissingCtx {
    #[deku(bits = 7)]
    a: u8,
    #[deku(bit_field)]
    b: NeedsCtx,
}

// Alone, the field is not batched, but `bit_field` still requires the trait.
#[derive(DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct NotABitField {
    #[deku(bit_field)]
    a: u16,
}

fn main() {}
