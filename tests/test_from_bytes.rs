#![cfg(feature = "bits")]

use deku::prelude::*;

#[test]
fn test_from_bytes_struct() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    struct TestDeku(#[deku(bits = 4)] u8);

    let test_data: Vec<u8> = [0b0110_0110u8, 0b0101_1010u8].to_vec();

    let ((rest, i), ret_read) = TestDeku::from_bytes((&test_data, 0)).unwrap();
    assert_eq!(TestDeku(0b0110), ret_read);
    assert_eq!(2, rest.len());
    assert_eq!(4, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((rest, i)).unwrap();
    assert_eq!(TestDeku(0b0110), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(0, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((rest, i)).unwrap();
    assert_eq!(TestDeku(0b0101), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(4, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((rest, i)).unwrap();
    assert_eq!(TestDeku(0b1010), ret_read);
    assert_eq!(0, rest.len());
    assert_eq!(0, i);
}

#[test]
fn test_from_bytes_enum() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = 4)]
    enum TestDeku {
        #[deku(id = "0b0110")]
        VariantA(#[deku(bits = 4)] u8),
        #[deku(id = "0b0101")]
        VariantB(#[deku(bits = 2)] u8),
    }

    let test_data: Vec<u8> = [0b0110_0110u8, 0b0101_1010u8].to_vec();

    let ((rest, i), ret_read) = TestDeku::from_bytes((&test_data, 0)).unwrap();
    assert_eq!(TestDeku::VariantA(0b0110), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(0, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((rest, i)).unwrap();
    assert_eq!(TestDeku::VariantB(0b10), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(6, i);
    assert_eq!(0b0101_1010u8, rest[0]);
}

#[test]
fn test_from_bytes_long() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = 4)]
    enum TestDeku {
        #[deku(id = "0b0110")]
        VariantA(#[deku(bits = 4)] u8),
        #[deku(id = "0b0101")]
        VariantB(#[deku(bits = 2)] u8),
    }

    let mut test_data = vec![0x00; 200];
    test_data.extend([0b0110_0110u8, 0b0101_1010u8].to_vec());

    let ((rest, i), ret_read) = TestDeku::from_bytes((&test_data, 200 * 8)).unwrap();
    assert_eq!(TestDeku::VariantA(0b0110), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(0, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((rest, i)).unwrap();
    assert_eq!(TestDeku::VariantB(0b10), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(6, i);
    assert_eq!(0b0101_1010u8, rest[0]);
}

#[test]
fn test_from_bytes_short_with_seek() {
    #[derive(Debug, DekuRead, Eq, PartialEq)]
    #[deku(ctx = "mid: u8", id = "mid")]
    enum Sneaky {
        #[deku(id = 0)]
        Zero,
        #[deku(id = 1)]
        One,
    }

    #[derive(Debug, DekuRead, Eq, PartialEq)]
    struct Seeky {
        id: u8,
        // Use seek to force an out-of-bounds slice for remaining data
        #[deku(seek_from_current = "1")]
        #[deku(ctx = "*id")]
        body: Sneaky,
    }

    let bytes: [u8; 1] = [0x01];
    assert_eq!(
        Err(DekuError::Incomplete(NeedSize::new(8))),
        Seeky::from_bytes((&bytes, 0))
    );
}
