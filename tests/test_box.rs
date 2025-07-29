#![cfg(feature = "std")]

use deku::prelude::*;

#[derive(DekuRead, DekuWrite)]
struct TestStruct {
    field: Box<u8>,
}

#[test]
fn test_box_smoke_test() {
    let test_data: &[u8] = &[0xf0];
    let a = TestStruct::try_from(test_data).unwrap();
    let new_bytes = a.to_bytes().unwrap();
    assert_eq!(test_data, &*new_bytes);
}
