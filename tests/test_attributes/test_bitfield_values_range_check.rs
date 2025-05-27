use std::io::Cursor;

use deku::prelude::*;

#[test]
fn check_big_unsigned_u10_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "big")]
        pub b: u16,
    }

    let buffer = vec![0b10000000, 0b00000010];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = 1023;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = 1024;
    assert!(test_struct.to_bytes().is_err());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_little_unsigned_u10_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "little")]
        pub b: u16,
    }

    let buffer = vec![0b10000000, 0b00001000];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = 1023;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = 1024;
    assert!(test_struct.to_bytes().is_err());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_big_signed_i10_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "big")]
        pub b: i16,
    }

    let buffer = vec![0b10000000, 0b00000010];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = 511;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = 512;
    assert!(test_struct.to_bytes().is_err());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_little_signed_i10_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "little")]
        pub b: i16,
    }

    let buffer = vec![0b10000000, 0b00001000];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = 511;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = 512;
    assert!(test_struct.to_bytes().is_err());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_big_signed_i10_decode_encode_negative_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "big")]
        pub b: i16,
    }

    let buffer = vec![0b10000011, 0b11111110];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, -2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = -512;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = -513;
    assert!(test_struct.to_bytes().is_err());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, -2);
}

#[test]
fn check_little_signed_i10_decode_encode_negative_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "little")]
        pub b: i16,
    }

    let buffer = vec![0b10000011, 0b11111011];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, -2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = -512;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = -513;
    assert!(test_struct.to_bytes().is_err());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, -2);
}

#[test]
fn check_big_unsigned_u16_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "15", bits = "16", endian = "big")]
        pub b: u16,
    }

    let buffer = vec![0b10000000, 0b00000000, 0b00000000, 0b00000010];

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = 0xFFFF;
    assert!(test_struct.to_bytes().is_ok());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_little_unsigned_u16_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "15", bits = "16", endian = "little")]
        pub b: u16,
    }

    let buffer = vec![0b10000000, 0b00000000, 0b00000010, 0b00000000];

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = 0xFFFF;
    assert!(test_struct.to_bytes().is_ok());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_big_signed_i16_decode_encode_positive_negative_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "15", bits = "16", endian = "big")]
        pub b: i16,
    }

    let buffer = vec![0b10000000, 0b00000000, 0b00000000, 0b00000010];

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = i16::MIN;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = i16::MAX;
    assert!(test_struct.to_bytes().is_ok());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}

#[test]
fn check_little_signed_i16_decode_encode_positive_negative_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "15", bits = "16", endian = "little")]
        pub b: i16,
    }

    let buffer = vec![0b10000000, 0b00000000, 0b00000010, 0b00000000];

    let ((remaining_bytes, offset), mut test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"));

    test_struct.b = i16::MIN;
    assert!(test_struct.to_bytes().is_ok());

    test_struct.b = i16::MAX;
    assert!(test_struct.to_bytes().is_ok());

    let mut cursor = Cursor::new(buffer);
    let (_, val) = TestStruct::from_reader((&mut cursor, 0)).unwrap();
    assert!(val.a);
    assert_eq!(val.b, 2);
}
