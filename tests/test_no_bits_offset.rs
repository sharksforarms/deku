#![cfg(not(feature = "bits"))]

use deku::prelude::*;

#[derive(DekuRead)]
struct Packet {
    value: u8,
}

#[test]
fn nonzero_byte_offset_returns_an_error_without_bits() {
    let result = Packet::from_bytes((&[0][..], 1));

    assert!(matches!(result, Err(DekuError::InvalidParam(_))));
}

#[test]
fn direct_bit_skips_return_an_error_without_bits() {
    let mut reader = Reader::new(no_std_io::io::Cursor::new([0u8]));

    let result = reader.skip_bits(1, deku::ctx::Order::Msb0);

    assert!(matches!(result, Err(DekuError::InvalidParam(_))));
}
