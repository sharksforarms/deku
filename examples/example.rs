use deku::prelude::*;
use std::convert::TryFrom;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct FieldF {
    #[deku(bits = "6")]
    data: u8,
}

/// DekuTest Struct
//   0                   1                   2                   3                   4
//   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |    field_a    |   field_b   |c|            field_d              | e |     f     |
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "little")] // By default it uses the system endianess, but can be overwritten
struct DekuTest {
    field_a: u8,
    #[deku(bits = "7")]
    field_b: u8,
    #[deku(bits = "1")]
    field_c: u8,
    #[deku(endian = "big")]
    field_d: u16,
    #[deku(bits = "2")]
    field_e: u8,
    field_f: FieldF,
    num_items: u8,
    #[deku(vec_len="num_items")]
    items: Vec<u16>,
}

fn main() {
    
    let test_data: &[u8] = [0xAB, 0b1010010_1, 0xAB, 0xCD, 0b1100_0110, 0x01, 0xBE,0xEF].as_ref();
    println!("{}/{}", test_data.len()*8, DekuTest::bit_size());
    
    let test_deku = DekuTest::try_from(test_data).unwrap();

    assert_eq!(
        DekuTest {
            field_a: 0xAB,
            field_b: 0b0_1010010,
            field_c: 0b0000000_1,
            field_d: 0xCDAB,
            field_e: 0b0000_0011,
            field_f: FieldF { data: 0b00_000110 },
            num_items: 1,
            items: vec![0xBEEF],
        },
        test_deku
    );

    let test_deku: Vec<u8> = test_deku.into();
    assert_eq!(test_data.to_vec(), test_deku);
}
