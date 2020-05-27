use deku::prelude::*;

#[test]
fn test_from_bytes() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    struct TestDeku(#[deku(bits = 4)] u8);

    let test_data: Vec<u8> = [0b0110_0110u8, 0b0101_1010u8].to_vec();

    let ((rest, i), ret_read) = TestDeku::from_bytes((&test_data, 0)).unwrap();
    assert_eq!(TestDeku(0b0110), ret_read);
    assert_eq!(2, rest.len());
    assert_eq!(4, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((&rest, i)).unwrap();
    assert_eq!(TestDeku(0b0110), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(0, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((&rest, i)).unwrap();
    assert_eq!(TestDeku(0b0101), ret_read);
    assert_eq!(1, rest.len());
    assert_eq!(4, i);

    let ((rest, i), ret_read) = TestDeku::from_bytes((&rest, i)).unwrap();
    assert_eq!(TestDeku(0b1010), ret_read);
    assert_eq!(0, rest.len());
    assert_eq!(0, i);
}
