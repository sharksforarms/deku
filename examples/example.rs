//! To test out the "logging" feature:
//! ```
//! $ RUST_LOG=trace cargo run --example example --features logging
//! ```

#![allow(clippy::unusual_byte_groupings)]

use core::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct FieldF {
    #[deku(bits = 6)]
    #[deku(assert_eq = "6")]
    data: u8,
}

/// DekuTest Struct
//   0                   1                   2                   3                   4
//   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |    field_a    |   field_b   |c|            field_d              | e |     f     |
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "little")] // By default it uses the system endianness, but can be overwritten
struct DekuTest {
    field_a: u8,
    #[deku(bits = 7)]
    field_b: u8,
    #[deku(bits = 1)]
    field_c: u8,
    #[deku(endian = "big")]
    field_d: u16,
    #[deku(bits = 2)]
    field_e: u8,
    field_f: FieldF,
    num_items: u8,
    #[deku(count = "num_items", endian = "big")]
    items: Vec<u16>,
}

fn main() {
    env_logger::init();
    let test_data: &[u8] = &[
        0xab,
        0b1010010_1,
        0xab,
        0xcd,
        0b1100_0110,
        0x02,
        0xbe,
        0xef,
        0xc0,
        0xfe,
    ];

    let test_deku = DekuTest::try_from(test_data).unwrap();

    println!("{test_deku:02x?}");
    assert_eq!(
        DekuTest {
            field_a: 0xab,
            field_b: 0b0_1010010,
            field_c: 0b0000000_1,
            field_d: 0xabcd,
            field_e: 0b0000_0011,
            field_f: FieldF { data: 0b00_000110 },
            num_items: 2,
            items: vec![0xbeef, 0xc0fe],
        },
        test_deku
    );

    let test_deku: Vec<u8> = test_deku.try_into().unwrap();
    assert_eq!(test_data.to_vec(), test_deku);
}
