use core::convert::TryFrom;

use deku::DekuError;
use deku::no_std_io::{Error, ErrorKind};
use deku::prelude::NeedSize;
use rstest::rstest;

#[rstest(
    bits,
    expected_bytes,
    case::empty(0, 0),
    case::one_bit(1, 1),
    case::partial_byte(7, 1),
    case::one_byte(8, 1),
    case::partial_second_byte(9, 2),
    case::two_bytes(16, 2)
)]
fn test_need_size(bits: usize, expected_bytes: usize) {
    let size = NeedSize::new(bits);
    assert_eq!(size.bit_size(), bits);
    assert_eq!(size.byte_size(), expected_bytes);
}

#[test]
fn test_error_from_core_conversion_errors() {
    let int_error = u8::try_from(256usize).unwrap_err();
    assert!(matches!(DekuError::from(int_error), DekuError::Parse(_)));

    let slice_error = <[u8; 2]>::try_from(&[0u8][..]).unwrap_err();
    assert!(matches!(DekuError::from(slice_error), DekuError::Parse(_)));
}

#[test]
fn test_error_from_io_error() {
    let error = Error::new(ErrorKind::BrokenPipe, "read failed");
    assert_eq!(DekuError::from(error), DekuError::Io(ErrorKind::BrokenPipe));
}

#[cfg(feature = "std")]
#[rstest(error, expected_kind, expected_message,
    case::incomplete(
        DekuError::Incomplete(NeedSize::new(9)),
        std::io::ErrorKind::UnexpectedEof,
        "Not enough data, need 9 bits (or 2 bytes)"
    ),
    case::parse(
        deku::deku_error!(DekuError::Parse, "parse"),
        std::io::ErrorKind::InvalidData,
        "Parse error: parse"
    ),
    case::invalid_param(
        deku::deku_error!(DekuError::InvalidParam, "invalid"),
        std::io::ErrorKind::InvalidInput,
        "Invalid param error: invalid"
    ),
    case::assertion(
        deku::deku_error!(DekuError::Assertion, "assertion"),
        std::io::ErrorKind::InvalidData,
        "assertion"
    ),
    case::missing_id(
        DekuError::IdVariantNotFound,
        std::io::ErrorKind::NotFound,
        "Could not resolve `id` for variant"
    ),
    case::io(
        DekuError::Io(ErrorKind::BrokenPipe),
        std::io::ErrorKind::BrokenPipe,
        "io error: BrokenPipe"
    ),
)]
fn test_error_to_std_io_error(
    error: DekuError,
    expected_kind: std::io::ErrorKind,
    expected_message: &str,
) {
    let io_error: std::io::Error = error.into();
    assert_eq!(io_error.kind(), expected_kind);
    assert_eq!(io_error.to_string(), expected_message);
}

#[cfg(not(feature = "std"))]
#[test]
fn test_error_to_no_std_io_error() {
    let cases = [
        (
            DekuError::Incomplete(NeedSize::new(9)),
            ErrorKind::UnexpectedEof,
        ),
        (
            deku::deku_error!(DekuError::Parse, "parse"),
            ErrorKind::InvalidData,
        ),
        (
            deku::deku_error!(DekuError::InvalidParam, "invalid"),
            ErrorKind::InvalidInput,
        ),
        (
            deku::deku_error!(DekuError::Assertion, "assertion"),
            ErrorKind::InvalidData,
        ),
        (DekuError::IdVariantNotFound, ErrorKind::NotFound),
        (DekuError::Io(ErrorKind::BrokenPipe), ErrorKind::BrokenPipe),
    ];

    for (error, expected_kind) in cases {
        let io_error: deku::no_std_io::Error = error.into();
        assert_eq!(io_error.kind(), expected_kind);
    }
}
