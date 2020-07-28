use deku::prelude::*;

#[derive(DekuRead)]
#[deku(id_type = "u8")]
enum Test1 {
    #[deku(id = "1", id_pat = "2..=3")] A(u8),
}

fn main() {}
