use deku::prelude::*;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_temp_field_write() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct {
        #[deku(temp, temp_value = "(self.field_b.len() as _)")]
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();
    let test_struct = TestStruct {
        field_b: vec![0x02],
    };
    let ret_write: Vec<u8> = test_struct.to_bytes().unwrap();

    assert_eq!(test_data, ret_write);
}

#[test]
fn test_temp_field_value_ignore_on_read() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct {
        #[deku(temp, temp_value = "100")]
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    }

    let test_data: Vec<u8> = [0x02, 0x02, 0x03].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_b: vec![0x02, 0x03]
        },
        ret_read
    );
}

#[test]
fn test_temp_field() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct {
        #[deku(temp)]
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    }

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestStruct {
            field_b: vec![0x02]
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data[1..].to_vec(), ret_write);
}

#[test]
fn test_temp_field_unnamed() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct(#[deku(temp)] u8, #[deku(count = "field_0")] Vec<u8>);

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = TestStruct::try_from(test_data.as_ref()).unwrap();
    assert_eq!(TestStruct(vec![0x02]), ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data[1..].to_vec(), ret_write);
}

#[test]
fn test_temp_field_unnamed_write() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    struct TestStruct(
        #[deku(temp, temp_value = "(self.0.len() as _)")] u8,
        #[deku(count = "field_0")] Vec<u8>,
    );

    let test_data: Vec<u8> = [0x01, 0x02].to_vec();
    let test_struct = TestStruct(vec![0x02]);
    let ret_write: Vec<u8> = test_struct.to_bytes().unwrap();

    assert_eq!(test_data, ret_write);
}

#[test]
fn test_temp_enum_field() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    #[deku(type = "u8")]
    enum TestEnum {
        #[deku(id = "0xAB")]
        VarA {
            #[deku(temp)]
            field_a: u8,
            #[deku(count = "field_a")]
            field_b: Vec<u8>,
        },
    }

    let test_data: Vec<u8> = [0xAB, 0x01, 0x02].to_vec();

    let ret_read = TestEnum::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        TestEnum::VarA {
            field_b: vec![0x02]
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0xAB, 0x02], ret_write);
}

#[test]
fn test_temp_enum_field_write() {
    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(PartialEq, Debug)]
    #[deku(type = "u8")]
    enum TestEnum {
        #[deku(id = "0xAB")]
        VarA {
            #[deku(
                temp,
                temp_value = "if let TestEnum::VarA { field_b } = self { field_b.len() as _ } else { unreachable!() };"
            )]
            field_a: u8,
            #[deku(count = "field_a")]
            field_b: Vec<u8>,
        },
        #[deku(id = "0xBA")]
        VarB(u8),
    }

    let test_data: Vec<u8> = [0xAB, 0x01, 0x02].to_vec();
    let ret_write: Vec<u8> = TestEnum::VarA {
        field_b: vec![0x02],
    }
    .to_bytes()
    .unwrap();
    assert_eq!(test_data, ret_write);

    let test_data: Vec<u8> = [0xBA, 0x10].to_vec();
    let ret_write: Vec<u8> = TestEnum::VarB(0x10).to_bytes().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_context() {
    use deku::prelude::*;
    use std::convert::TryInto;

    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(Debug, PartialEq)]
    #[deku(ctx = "_field1: u8, _field2: u8")]
    struct Child {}

    #[deku_derive(DekuRead, DekuWrite)]
    #[derive(Debug, PartialEq)]
    struct Parent {
        pub field1: u8,

        #[deku(temp, temp_value = "self.field1")]
        pub field2: u8,

        #[deku(ctx = "*field1, *field2")]
        pub field3: Child,
    }

    let value = Parent {
        field1: 0x01,
        field3: Child {},
    };
    let value: Vec<u8> = value.try_into().unwrap();
    assert_eq!(vec![0x01, 0x01], value);
}
