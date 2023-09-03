use std::convert::{TryFrom, TryInto};

use deku::prelude::*;

#[test]
fn test_bound() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(bound = "for<'a> T: DekuRead<'a, ()> + DekuWrite<()>")]
    struct GenericType<T>(T);

    let test_data = [0x01_u8];
    let ret_read = <GenericType<u8>>::try_from(&test_data[..]).unwrap();
    assert_eq!(ret_read, GenericType(0x01));

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);
}
