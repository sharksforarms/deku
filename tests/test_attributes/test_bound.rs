use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;

use deku::prelude::*;

#[test]
fn test_bound() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(bound = "T: DekuRead<'a, ()>, T: DekuWrite<()>")]
    struct GenericType<'a, T> {
        t: T,
        #[deku(skip)]
        phantom: PhantomData<&'a ()>,
    }

    let test_data = [0x01_u8];
    let ret_read = <GenericType<u8>>::try_from(&test_data[..]).unwrap();
    assert_eq!(
        ret_read,
        GenericType {
            t: 0x01,
            phantom: PhantomData
        }
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);
}
