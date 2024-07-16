use deku::prelude::*;

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct A {
    #[deku(bytes = "3", endian = "big")]
    address: u32,
}

#[test]
fn test_to_slice_bytes() {
    let bytes = [0x11, 0x22, 0x33];
    let a = A::from_bytes((&bytes, 0)).unwrap().1;
    let new_bytes = a.to_bytes().unwrap();
    assert_eq!(bytes, &*new_bytes);

    let bytes = [0x11, 0x22, 0x33];
    let mut out = [0x00; 3];
    let amt_written = a.to_slice(&mut out).unwrap();
    assert_eq!(bytes, out.as_slice());
    assert_eq!(amt_written, 3);
}

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct B {
    #[deku(bits = "3", endian = "little")]
    inner: u8,
}

#[test]
fn test_to_slice_bits() {
    let b = B { inner: 0b111 };

    let mut out = [0x00; 1];
    let amt_written = b.to_slice(&mut out).unwrap();
    assert_eq!([0b1110_0000], out.as_slice());
    assert_eq!(amt_written, 1);
}

#[test]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: Io(WriteZero)")]
fn test_to_slice_panic_writer_failure() {
    let bytes = [0x11, 0x22, 0x33];
    let a = A::from_bytes((&bytes, 0)).unwrap().1;

    let mut out = [0x00; 2];
    a.to_slice(&mut out).unwrap();
}
