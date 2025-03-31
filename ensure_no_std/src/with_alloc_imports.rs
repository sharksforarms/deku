use alloc::vec;
use alloc::vec::Vec;
use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = 5)]
    field_a: u8,
    #[deku(bits = 3)]
    field_b: u8,
    count: u8,
    #[deku(count = "count", pad_bytes_after = "8")]
    data: Vec<u8>,
    #[deku(count = "1")]
    after: Vec<u8>,
}

pub fn rw() {
    #[allow(clippy::unusual_byte_groupings)]
    let test_data: &[u8] = &[0b10101_101, 0x02, 0xBE, 0xEF, 0xff, 0xaa];
    let mut cursor = deku::no_std_io::Cursor::new(test_data);

    // Test reading
    let (_rest, val) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(
        DekuTest {
            field_a: 0b10101,
            field_b: 0b101,
            count: 0x02,
            data: vec![0xBE, 0xEF],
            after: vec![0xaa],
        },
        val
    );

    // Test writing
    let _val = val.to_bytes().unwrap();
}
