use deku::prelude::*;

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct A {
    #[deku(bytes = "3", endian = "big")]
    address: u32,
}

#[test]
#[cfg(feature = "alloc")]
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

#[cfg(feature = "bits")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct B {
    #[deku(bits = "3", endian = "little")]
    inner: u8,
}

#[test]
#[cfg(feature = "bits")]
fn test_to_slice_bits() {
    let b = B { inner: 0b111 };

    let mut out = [0x00; 1];
    let amt_written = b.to_slice(&mut out).unwrap();
    assert_eq!([0b1110_0000], out.as_slice());
    assert_eq!(amt_written, 1);
}

#[test]
#[cfg(feature = "alloc")]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: Io(WriteZero)")]
fn test_to_slice_panic_writer_failure() {
    let bytes = [0x11, 0x22, 0x33];
    let a = A::from_bytes((&bytes, 0)).unwrap().1;

    let mut out = [0x00; 2];
    a.to_slice(&mut out).unwrap();
}

#[cfg(feature = "bits")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct C {
    a: u8,
    #[deku(bits = "3", endian = "little")]
    b: u8,
    c: u8,
    #[deku(bits = "13", endian = "little")]
    d: u16,
    e: u8,
}

#[test]
#[cfg(feature = "bits")]
fn test_to_slice_counts_unaligned_writes_with_aligned_end() {
    let bytes = [0x11, 0x22, 0x33, 0x44, 0x55];
    let (leftover, c) = C::from_bytes((&bytes, 0)).unwrap();
    assert_eq!(leftover, (&[][..], 0));

    let mut out = [0x00; 5];
    let amt_written = c.to_slice(&mut out).unwrap();
    assert_eq!(bytes, out);
    assert_eq!(bytes.len(), amt_written);
}

#[cfg(feature = "bits")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct D {
    a: u8,
    #[deku(bits = "3", endian = "little")]
    b: u8,
    c: u8,
    #[deku(bits = "12", endian = "little")]
    d: u16,
    e: u8,
}

#[test]
#[cfg(feature = "bits")]
fn test_to_slice_counts_unaligned_writes_with_unaligned_end() {
    let bytes = [0x11, 0x22, 0x33, 0x44, 0x54];
    let (leftover, d) = D::from_bytes((&bytes, 0)).unwrap();
    assert_eq!(leftover, (&[0x54][..], 7));

    let mut out = [0x00; 5];
    let amt_written = d.to_slice(&mut out).unwrap();
    assert_eq!(bytes, out);
    assert_eq!(bytes.len(), amt_written);
}
