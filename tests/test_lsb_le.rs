use assert_hex::assert_eq_hex;
use deku::prelude::*;

#[test]
fn test_lsb_le_misaligned_1() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(bit_order = "lsb", endian = "little")]
    pub struct TestStruct {
        #[deku(bits = 1)]
        pub field_a: u8, // Bit 0
        #[deku(bits = 1)]
        pub field_b: u8, // Bit 1
        #[deku(bits = 1)]
        pub field_c: u8, // Bit 2
        #[deku(bits = 27)]
        pub field_d: u32, // Bits 3-29
        #[deku(bits = 2)]
        pub field_e: u8, // Bits 30-31
    }

    // Interpreted as Little Endian:
    // 31      23        15         7        0
    // |-0xF1--| |-0x32--| |-0xDC--| |-0xAD--|
    // 1111 0001 0011 0010 1101 1100 1010 1101
    //   11 0001 1101 1100 0011 0010 1010 1    - Value decoded by deku@0902ae0
    //   ^-----^ ^-----------------^ ^----^
    //    used        front_bits     leftover
    // EEDD DDDD DDDD DDDD DDDD DDDD DDDD DCBA
    const RAW_DATA: &[u8] = &[0xAD, 0xDC, 0x32, 0xF1];

    let parsed = TestStruct::from_bytes((RAW_DATA, 0)).unwrap().1;
    println!("{parsed:#02x?}");

    let manual = TestStruct {
        field_a: (u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_0001) as u8,
        field_b: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_0002) >> 1) as u8,
        field_c: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_0004) >> 2) as u8,
        field_d: (u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x3FFF_FFF8) >> 3,
        field_e: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xC000_0000) >> 30) as u8,
    };

    assert_eq_hex!(
        manual,
        TestStruct {
            field_a: 0x1,
            field_b: 0x0,
            field_c: 0x1,
            field_d: 0x6265B95,
            field_e: 0x3,
        },
        "Unexpected manual calculation"
    );
    assert_eq_hex!(parsed, manual, "Invalid deku calculation");
}

#[test]
fn test_lsb_le_misaligned_2() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(endian = "little", bit_order = "lsb")]
    struct Header {
        #[deku(bits = 20)]
        foo: u32,
        #[deku(bits = 12)]
        bar: u32,
    }
    let reference = Header {
        foo: 0xabc,
        bar: 0xdef,
    };
    let data = reference.to_bytes().unwrap();
    assert_eq!(data, b"\xbc\x0a\xf0\xde");
    let (_, actual) = Header::from_bytes((&data[..], 0)).unwrap();
    assert_eq!(actual, reference);
}
