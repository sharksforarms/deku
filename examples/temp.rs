use deku::bitvec::{BitVec, Msb0};
use deku::prelude::*;
use std::convert::TryInto;

#[deku_derive(DekuRead, DekuWrite)]
#[derive(Debug, PartialEq)]
struct DekuTest {
    a: u8,
    b: u8,

    #[deku(writer = "checksum1(deku::output)")]
    #[deku(temp)]
    sum1: (),

    #[deku(count = "2")]
    data: Vec<u8>,

    #[deku(writer = "checksum2(deku::output)")]
    #[deku(temp)]
    sum2: (),
}

fn checksum1(output: &mut BitVec<u8, Msb0>) -> Result<(), DekuError> {
    let sum: u16 = output.as_raw_slice().iter().map(|v| *v as u16).sum();
    sum.write(output, ())
}

fn checksum2(output: &mut BitVec<u8, Msb0>) -> Result<(), DekuError> {
    let sum: u32 = output.as_raw_slice().iter().map(|v| *v as u32).sum();
    sum.write(output, ())
}

fn main() {
    let test_data_read: &[u8] = [
        1, 2, // a and b as u8
        1, 2, // data as u8
    ]
    .as_ref();

    let test_data_write: &[u8] = [
        1, 2, // a and b as u8
        3, 0, // sum1 as u16 little endian (1+2 == 3)
        1, 2, // data as u8
        9, 0, 0, 0, // sum2 as u32 little endian (1+2+3+1+2 == 9)
    ]
    .as_ref();

    let (_rest, ret_read) = DekuTest::from_bytes((test_data_read, 0)).unwrap();

    assert_eq!(
        ret_read,
        DekuTest {
            a: 1,
            b: 2,
            data: vec![1, 2],
        }
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data_write.to_vec(), ret_write);
}
