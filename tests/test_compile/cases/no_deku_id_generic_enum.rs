use deku::prelude::*;

#[derive(DekuRead)]
#[deku(id_type = "u8")]
pub enum Body<T: for<'a> DekuReader<'a>> {
    #[deku(id = "0x0001")]
    First(T),
}

fn main() {
    let n = Body::<u8>::First(1);
    n.deku_id();
}
