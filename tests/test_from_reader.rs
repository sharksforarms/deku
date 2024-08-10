use deku::prelude::*;
use no_std_io::io::Seek;

#[cfg(feature = "bits")]
#[test]
fn test_from_reader_struct() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    struct TestDeku(#[deku(bits = 4)] u8);

    let test_data: Vec<u8> = [0b0110_0110u8, 0b0101_1010u8].to_vec();
    let mut c = std::io::Cursor::new(test_data);

    c.rewind().unwrap();
    let (amt_read, ret_read) = TestDeku::from_reader((&mut c, 0)).unwrap();
    assert_eq!(amt_read, 4);
    let mut total_read = amt_read;
    assert_eq!(TestDeku(0b0110), ret_read);

    c.rewind().unwrap();
    let (amt_read, ret_read) = TestDeku::from_reader((&mut c, total_read)).unwrap();
    assert_eq!(amt_read, 8);
    total_read = amt_read;
    assert_eq!(TestDeku(0b0110), ret_read);

    c.rewind().unwrap();
    let (amt_read, ret_read) = TestDeku::from_reader((&mut c, total_read)).unwrap();
    assert_eq!(amt_read, 12);
    total_read = amt_read;
    assert_eq!(TestDeku(0b0101), ret_read);

    c.rewind().unwrap();
    let (amt_read, ret_read) = TestDeku::from_reader((&mut c, total_read)).unwrap();
    assert_eq!(amt_read, 16);
    assert_eq!(TestDeku(0b1010), ret_read);
}

#[cfg(feature = "bits")]
#[test]
fn test_from_reader_enum() {
    #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = 4)]
    enum TestDeku {
        #[deku(id = "0b0110")]
        VariantA(#[deku(bits = 4)] u8),
        #[deku(id = "0b0101")]
        VariantB(#[deku(bits = 2)] u8),
    }

    let test_data = [0b0110_0110u8, 0b0101_1010u8];
    let mut c = std::io::Cursor::new(test_data);

    let (first_amt_read, ret_read) = TestDeku::from_reader((&mut c, 0)).unwrap();
    assert_eq!(first_amt_read, 8);
    assert_eq!(TestDeku::VariantA(0b0110), ret_read);
    c.rewind().unwrap();

    let (amt_read, ret_read) = TestDeku::from_reader((&mut c, first_amt_read)).unwrap();
    assert_eq!(amt_read, 6 + first_amt_read);
    assert_eq!(TestDeku::VariantB(0b10), ret_read);
}
