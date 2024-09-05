#[derive(PartialEq, Debug, deku::DekuRead, deku::DekuWrite)]
#[deku(id_type = "u8")]
enum TestEnum {
    #[deku(id = "1")]
    VarA(u8),
    #[deku(id = "2")]
    VarB(#[deku(bits = 4)] u8, #[deku(bits = 4)] u8),
    #[deku(id = "3")]
    VarC {
        #[deku(update = "field_b.len()")]
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    },
    #[deku(id = "4")]
    VarD(
        #[deku(update = "field_1.len()")] u8,
        #[deku(count = "field_0")] Vec<u8>,
    ),

    #[deku(id_pat = "_")]
    VarDefault { id: u8, value: u8 },
}

fn main() {
    
}