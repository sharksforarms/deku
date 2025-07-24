use core::convert::{TryFrom, TryInto};
use deku::prelude::*;

fn main() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        #[deku(read_all)]
        data: Vec<(u8, u8)>,
    }

    let test_data: Vec<u8> = [0xaa, 0xbb, 0xcc, 0xdd].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            data: vec![(0xaa, 0xbb), (0xcc, 0xdd)]
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}
