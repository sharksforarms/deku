use std::io::Cursor;

use deku::{prelude::*, reader::Reader};
use hexlit::hex;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id = "0")]
    Var1,
    #[deku(id = "1")]
    Var2(#[deku(bytes = "2")] u32),
    #[deku(id = "2")]
    Var3(u8, u8),
    #[deku(id = "3")]
    Var4 {
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    },
    #[deku(id_pat = "4..=6")]
    Var5 { id: u8 },
    #[deku(id_pat = "&id if id > 6")]
    Var6 { id: u8 },
    #[deku(id_pat = "_")]
    VarDefault { id: u8, value: u8 },
}

fn main() {
    let test_data = hex!("03020102").to_vec();

    let mut cursor = Cursor::new(&test_data);
    let mut reader = Reader::new(&mut cursor);
    let deku_test = DekuTest::from_reader_with_ctx(&mut reader, ()).unwrap();

    assert_eq!(
        DekuTest::Var4 {
            field_a: 0x02,
            field_b: vec![0x01, 0x02]
        },
        deku_test
    );

    let ret_out: Vec<u8> = deku_test.to_bytes().unwrap();

    assert_eq!(test_data, ret_out);

    let id_first_byte = deku_test.deku_id();
    assert_eq!(Ok(test_data[0]), id_first_byte);
}
