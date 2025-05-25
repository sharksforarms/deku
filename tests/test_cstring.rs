use deku::prelude::*;

use std::ffi::CString;

#[test]
fn test_cstring_no_ctx() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct Data {
        s: CString,
    }

    let bytes = &[b't', b'e', b's', b't', b'\0'];
    let (_, d) = Data::from_bytes((bytes, 0)).unwrap();
    assert_eq!(d.s, CString::new("test").unwrap());
}

#[test]
fn test_cstring_bytes() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct Data {
        len: u8,
        #[deku(bytes = "*len as usize")]
        s: CString,
    }

    let bytes = &[0x05, b't', b'e', b's', b't', b'\0'];
    let (_, d) = Data::from_bytes((bytes, 0)).unwrap();
    assert_eq!(d.s, CString::new("test").unwrap());
}

#[test]
fn test_cstring_valid_bytes() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct Data {
        #[deku(bytes = "5")]
        s: CString,
    }

    let bytes = &[b't', b'e', b's', b't', b'\0'];
    let (_, d) = Data::from_bytes((bytes, 0)).unwrap();
    assert_eq!(d.s, CString::new("test").unwrap());
}

#[should_panic(
    expected = "Failed to convert Vec to CString: data provided contains an interior nul byte at pos 4"
)]
#[test]
fn test_cstring_trailing_nul() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct Data {
        #[deku(bytes = "6")]
        s: CString,
    }

    let bytes = &[b't', b'e', b's', b't', b'\0', b'\0'];
    let (_, d) = Data::from_bytes((bytes, 0)).unwrap();
    assert_eq!(d.s, CString::new("test").unwrap());
}
