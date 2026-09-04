use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite, DekuSize)]
struct DekuTest {
    field_a: u8,
    field_b: u8,
    count: u8,
}

pub fn rw() {
    #[allow(clippy::unusual_byte_groupings)]
    let test_data: &[u8] = &[0xaa, 0xb0, 0x02];

    // Test reading
    let (_rest, val) = DekuTest::from_bytes((test_data, 0)).unwrap();
    assert_eq!(
        DekuTest {
            field_a: 0xaa,
            field_b: 0xb0,
            count: 0x02,
        },
        val
    );

    // Test writing
    const BUF_SIZE: usize = DekuTest::SIZE_BYTES.unwrap();
    let mut buf = [0; BUF_SIZE];
    let _val = val.to_slice(&mut buf).unwrap();
}
