use deku::prelude::*;
use hex_literal::hex;
use std::convert::TryFrom;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id = "0")]
    VarD,
    #[deku(id = "1")]
    Var1(#[deku(bytes = "2")] u32),
    #[deku(id = "2")]
    Var2(u8, u8),
    #[deku(id = "3")]
    Var3 {
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    },
}

fn main() {
    let test_data = hex!("03020102").to_vec();

    let deku_test = DekuTest::try_from(test_data.as_ref()).unwrap();

    assert_eq!(
        DekuTest::Var3 {
            field_a: 0x02,
            field_b: vec![0x01, 0x02]
        },
        deku_test
    );

    let ret_out: Vec<u8> = deku_test.to_bytes().unwrap();

    assert_eq!(test_data, ret_out);
}
