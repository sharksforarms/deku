#![cfg(feature = "bits")]

use assert_hex::assert_eq_hex;
use deku::prelude::*;

#[test]
fn test_lsb_le_misaligned_middle() {
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

    let expected = TestStruct {
        field_a: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_0001) as u8),
        field_b: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_0002) >> 1) as u8,
        field_c: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_0004) >> 2) as u8,
        field_d: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x3FFF_FFF8) >> 3),
        field_e: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xC000_0000) >> 30) as u8,
    };

    assert_eq_hex!(
        expected,
        TestStruct {
            field_a: 0x1,
            field_b: 0x0,
            field_c: 0x1,
            field_d: 0x6265B95,
            field_e: 0x3,
        },
        "Incorrect manual calculation"
    );
    assert_eq_hex!(parsed, expected, "Invalid deku calculation");
}

#[test]
fn test_lsb_le_misaligned_right() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(bit_order = "lsb", endian = "little")]
    pub struct TestStruct {
        #[deku(bits = 5)]
        pub field_a: u8, // Bits 0-4
        #[deku(bits = 27)]
        pub field_b: u32, // Bits 5-31
    }

    const RAW_DATA: &[u8] = &[0xAD, 0xDC, 0x32, 0xF1];

    let parsed = TestStruct::from_bytes((RAW_DATA, 0)).unwrap().1;

    let expected = TestStruct {
        field_a: (u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_001F) as u8,
        field_b: (u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xFFFF_FFE0) >> 5,
    };

    assert_eq_hex!(
        expected,
        TestStruct {
            field_a: 0xD,
            field_b: 0x0789_96E5,
        },
        "Incorrect manual calculation"
    );
    assert_eq_hex!(parsed, expected, "Invalid deku calculation");
}

#[test]
fn test_lsb_le_misaligned_left() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(bit_order = "lsb", endian = "little")]
    pub struct TestStruct {
        #[deku(bits = 27)]
        pub field_a: u32, // Bits 0-26
        #[deku(bits = 5)]
        pub field_b: u8, // Bits 27-31
    }

    const RAW_DATA: &[u8] = &[0xAD, 0xDC, 0x32, 0xF1];

    let parsed = TestStruct::from_bytes((RAW_DATA, 0)).unwrap().1;

    let expected = TestStruct {
        field_a: (u32::from_le_bytes(RAW_DATA.try_into().unwrap())) & 0x07FF_FFFF,
        field_b: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xF800_0000) >> 27) as u8,
    };

    assert_eq_hex!(
        expected,
        TestStruct {
            field_a: 0x0132_DCAD,
            field_b: 0x1E,
        },
        "Incorrect manual calculation"
    );
    assert_eq_hex!(parsed, expected, "Invalid deku calculation");
}

#[test]
fn test_lsb_le_aligned_right() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(bit_order = "lsb", endian = "little")]
    pub struct TestStruct {
        #[deku(bits = 8)]
        pub field_a: u8, // Bits 0-7
        #[deku(bits = 24)]
        pub field_b: u32, // Bits 8-31
    }

    const RAW_DATA: &[u8] = &[0xAD, 0xDC, 0x32, 0xF1];

    let parsed = TestStruct::from_bytes((RAW_DATA, 0)).unwrap().1;

    let expected = TestStruct {
        field_a: (u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_00FF) as u8,
        field_b: (u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xFFFF_FF00) >> 8,
    };

    assert_eq_hex!(
        expected,
        TestStruct {
            field_a: 0xAD,
            field_b: 0x00F1_32DC,
        },
        "Incorrect manual calculation"
    );
    assert_eq_hex!(parsed, expected, "Invalid deku calculation");
}

#[test]
fn test_lsb_le_aligned_left() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(bit_order = "lsb", endian = "little")]
    pub struct TestStruct {
        #[deku(bits = 24)]
        pub field_a: u32, // Bits 0-23
        #[deku(bits = 8)]
        pub field_b: u8, // Bits 24-31
    }

    const RAW_DATA: &[u8] = &[0xAD, 0xDC, 0x32, 0xF1];

    let parsed = TestStruct::from_bytes((RAW_DATA, 0)).unwrap().1;

    let expected = TestStruct {
        field_a: (u32::from_le_bytes(RAW_DATA.try_into().unwrap())) & 0x00FF_FFFF,
        field_b: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xFF00_0000) >> 24) as u8,
    };

    assert_eq_hex!(
        expected,
        TestStruct {
            field_a: 0x0032_DCAD,
            field_b: 0xF1,
        },
        "Incorrect manual calculation"
    );
    assert_eq_hex!(parsed, expected, "Invalid deku calculation");
}

#[test]
fn test_lsb_le_aligned_mixed() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(bit_order = "lsb", endian = "little")]
    pub struct TestStruct {
        #[deku(bits = 16)]
        pub field_a: u16, // Bits 0-15
        #[deku(bits = 1)]
        pub field_b: u8, // Bit 16
        #[deku(bits = 2)]
        pub field_c: u8, // Bits 17-18
        #[deku(bits = 4)]
        pub field_d: u8, // Bits 19-22
        #[deku(bits = 6)]
        pub field_e: u8, // Bits 23-28
        #[deku(bits = 3)]
        pub field_f: u8, // Bits 29-31
    }

    const RAW_DATA: &[u8] = &[0xAD, 0xDC, 0x32, 0xF1];

    let parsed = TestStruct::from_bytes((RAW_DATA, 0)).unwrap().1;

    let expected = TestStruct {
        field_a: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0000_FFFF) as u16),
        field_b: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0001_0000) >> 16) as u8,
        field_c: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0006_0000) >> 17) as u8,
        field_d: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x0078_0000) >> 19) as u8,
        field_e: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0x1F80_0000) >> 23) as u8,
        field_f: ((u32::from_le_bytes(RAW_DATA.try_into().unwrap()) & 0xE000_0000) >> 29) as u8,
    };

    assert_eq_hex!(
        expected,
        TestStruct {
            field_a: 0xDCAD,
            field_b: 0x0,
            field_c: 0x1,
            field_d: 0x6,
            field_e: 0x22,
            field_f: 0x7,
        },
        "Incorrect manual calculation"
    );
    assert_eq_hex!(parsed, expected, "Invalid deku calculation");
}

#[test]
#[cfg(feature = "alloc")]
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

#[test]
#[cfg(feature = "alloc")]
fn test_invalid_lsb_bit_split_squashfs_v3() {
    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    #[deku(endian = "little", bit_order = "lsb")]
    pub struct Dir {
        #[deku(bits = "19")]
        pub file_size: u32,
        #[deku(bits = "13")]
        pub offset: u16,
    }

    let expected_file_size = 38u32;
    let expected_offset = 44u16;
    let combined = ((expected_offset as u32) << 19) | expected_file_size;
    let test_data = combined.to_le_bytes();
    let (_, result) = Dir::from_bytes((&test_data, 0)).unwrap();
    assert_eq!(result.file_size, expected_file_size);
    assert_eq!(result.offset, expected_offset);
    assert_eq!(test_data, &*result.to_bytes().unwrap());
}
