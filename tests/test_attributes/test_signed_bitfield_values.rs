use deku::prelude::*;

#[test]
fn check_signed_i10_decode_encode_positive_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "big")]
        pub b: i16,
    }

    let buffer = vec![0b10000000, 0b00000010];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, 2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"))
}

#[test]
fn check_signed_i10_decode_encode_negative_value() {
    #[derive(Debug, PartialEq, Default, Clone, DekuRead, DekuWrite)]
    pub struct TestStruct {
        #[deku(bits = "1")]
        pub a: bool,
        #[deku(pad_bits_before = "5", bits = "10", endian = "big")]
        pub b: i16,
    }

    let buffer = vec![0b10000011, 0b11111110];
    //                                 ^^    ^^^^^^^^10 bits

    let ((remaining_bytes, offset), test_struct) =
        TestStruct::from_bytes((&buffer, 0)).expect("decoder error");

    // everything consumed?
    assert_eq!(offset, 0);
    assert_eq!(remaining_bytes.len(), 0);

    // check content
    assert!(test_struct.a);
    assert_eq!(test_struct.b, -2);

    // write back and check
    assert_eq!(buffer, test_struct.to_bytes().expect("encode error"))
}
