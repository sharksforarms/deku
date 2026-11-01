use deku::prelude::*;

#[derive(DekuRead)]
struct Packet {
    #[deku(read_all, count = "1")]
    payload: Vec<u8>,
}

fn main() {}
