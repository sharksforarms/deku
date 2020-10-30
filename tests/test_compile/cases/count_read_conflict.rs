use deku::prelude::*;

#[derive(DekuRead)]
struct Test1 {
    #[deku(bits_read = "1", bytes_read = "2")]
    a: Vec<u8>,
}

#[derive(DekuRead)]
struct Test2 {
    #[deku(count = "1", bits_read = "1", bytes_read = "2")]
    a: Vec<u8>,
}

#[derive(DekuRead)]
struct Test3 {
    #[deku(count = "1", bits_read = "1")]
    a: Vec<u8>,
}

#[derive(DekuRead)]
struct Test4 {
    #[deku(count = "1", bytes_read = "2")]
    a: Vec<u8>,
}


fn main() {}
