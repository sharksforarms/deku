use core::convert::{TryFrom, TryInto};

use deku::prelude::*;

/// Skip
#[test]
fn test_skip() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip)]
        field_b: Option<u8>,
        field_c: u8,
    }

    // Skip `field_b`
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: None, // Default::default()
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

/// Skip and default
#[test]
fn test_skip_default() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip, default = "5")]
        field_b: u8,
        field_c: u8,
    }

    // Skip `field_b` and default it's value to 5
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x05,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

/// Conditional skipping
#[test]
fn test_skip_cond() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip, cond = "*field_a == 0x01", default = "5")]
        field_b: u8,
    }

    // if `cond` is true, skip and default `field_b` to 5
    let test_data: Vec<u8> = [0x01].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x05, // default
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);

    // if `cond` is false, read `field_b` from input
    let test_data: Vec<u8> = [0x02, 0x03].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x02,
            field_b: 0x03,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_skip_read() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(read), default = "42")]
        field_b: u8,
        field_c: u8,
    }

    let test_data: Vec<u8> = vec![0x01, 0x02];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 42,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01, 42, 0x02], ret_write);
}

#[test]
fn test_skip_read_with_conditional() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(read), cond = "*field_a == 0x01", default = "42")]
        field_b: u8,
        field_c: u8,
    }

    let test_data: Vec<u8> = vec![0x01, 0x02];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 42,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01, 42, 0x02], ret_write);
}

#[test]
fn test_skip_read_with_conditional_not_firing() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(read), cond = "*field_a == 0xff", default = "42")]
        field_b: u8,
        field_c: u8,
    }

    let test_data: Vec<u8> = vec![0x01, 0x0a, 0x02];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x0a,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01, 0x0a, 0x02], ret_write);
}

#[test]
fn test_skip_write() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(write))]
        field_b: u8,
        field_c: u8,
    }

    let test_data: Vec<u8> = vec![0x01, 0x02, 0x03];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x02,
            field_c: 0x03,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01, 0x03], ret_write);
}

#[test]
fn test_skip_write_with_conditional() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(write), cond = "*field_a == 0x01")]
        field_b: u8,
    }

    // field is always read
    let test_data: Vec<u8> = vec![0x01, 0x02];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 0x02,
        },
        ret_read
    );

    // When cond is true, field_b is not written
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01], ret_write);
}

#[test]
fn test_skip_write_with_conditional_not_firing() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(write), cond = "*field_a == 0x01")]
        field_b: u8,
    }

    // field is always read
    let test_data: Vec<u8> = vec![0x0a, 0x02];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x0a,
            field_b: 0x02,
        },
        ret_read
    );

    // When cond is false, field_b is written
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x0a, 0x02], ret_write);
}

#[test]
fn test_both_skip_modes() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct TestStruct {
        field_a: u8,
        #[deku(skip(read), default = "100")]
        field_b: u8,
        #[deku(skip(write))]
        field_c: u8,
        field_d: u8,
    }

    let test_data: Vec<u8> = vec![0x01, 0x02, 0x03];

    let ret_read = TestStruct::try_from(test_data.as_slice()).unwrap();
    assert_eq!(
        TestStruct {
            field_a: 0x01,
            field_b: 100,
            field_c: 0x02,
            field_d: 0x03,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01, 100, 0x03], ret_write);
}
