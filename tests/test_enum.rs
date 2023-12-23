use std::convert::{TryFrom, TryInto};

use deku::prelude::*;
use hexlit::hex;
use rstest::*;

/// General smoke tests for enums
/// TODO: These should be divided into smaller tests

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum TestEnum {
    #[deku(id = "1")]
    VarA(u8),
    #[deku(id = "2")]
    VarB(#[deku(bits = 4)] u8, #[deku(bits = 4)] u8),
    #[deku(id = "3")]
    VarC {
        #[deku(update = "field_b.len()")]
        field_a: u8,
        #[deku(count = "field_a")]
        field_b: Vec<u8>,
    },
    #[deku(id = "4")]
    VarD(
        #[deku(update = "field_1.len()")] u8,
        #[deku(count = "field_0")] Vec<u8>,
    ),

    #[deku(id_pat = "_")]
    VarDefault { id: u8, value: u8 },
}

#[rstest(input,expected,
    case(&hex!("01AB"), TestEnum::VarA(0xAB)),
    case(&hex!("0269"), TestEnum::VarB(0b0110, 0b1001)),
    case(&hex!("0302AABB"), TestEnum::VarC{field_a: 0x02, field_b: vec![0xAA, 0xBB]}),
    case(&hex!("0402AABB"), TestEnum::VarD(0x02, vec![0xAA, 0xBB])),
    case(&hex!("FF01"), TestEnum::VarDefault{id: 0xFF, value: 0x01}),

    #[should_panic(expected = "Parse(\"Too much data\")")]
    case(&hex!("FFFFFF"), TestEnum::VarA(0xFF)),
)]
fn test_enum(input: &[u8], expected: TestEnum) {
    let input = input.to_vec();
    let ret_read = TestEnum::try_from(input.as_slice()).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input, ret_write);
}

#[test]
#[should_panic(expected = "Parse(\"Could not match enum variant id = 2 on enum `TestEnum`\")")]
fn test_enum_error() {
    #[derive(DekuRead)]
    #[deku(id_type = "u8")]
    enum TestEnum {
        #[deku(id = "1")]
        VarA(u8),
    }

    let test_data = &mut [0x02, 0x02];
    let _ret_read = TestEnum::try_from(test_data.as_slice()).unwrap();
}

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum TestEnumDiscriminant {
    VarA = 0x00,
    VarB,
    VarC = 0x02,
}

#[rstest(input, expected,
    case(&hex!("00"), TestEnumDiscriminant::VarA),
    case(&hex!("01"), TestEnumDiscriminant::VarB),
    case(&hex!("02"), TestEnumDiscriminant::VarC),

    #[should_panic(expected = "Could not match enum variant id = 3 on enum `TestEnumDiscriminant`")]
    case(&hex!("03"), TestEnumDiscriminant::VarA),
)]
fn test_enum_discriminant(input: &[u8], expected: TestEnumDiscriminant) {
    let input = input.to_vec();
    let ret_read = TestEnumDiscriminant::try_from(input.as_slice()).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input, ret_write);
}

#[test]
fn test_enum_array_type() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(id_type = "[u8; 3]")]
    enum TestEnumArray {
        #[deku(id = b"123")]
        VarA,
        #[deku(id = "[1,1,1]")]
        VarB,
    }

    let input = b"123".to_vec();

    let ret_read = TestEnumArray::try_from(input.as_slice()).unwrap();
    assert_eq!(TestEnumArray::VarA, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);
}

#[test]
fn test_id_pat_with_id() {
    // In these tests, the id_pat is already stored in the previous read to `my_id`, so we don't
    // use that for the next reading...

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct DekuTest {
        my_id: u8,
        #[deku(ctx = "*my_id")]
        enum_from_id: MyEnum,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "my_id: u8", id = "my_id")]
    pub enum MyEnum {
        #[deku(id_pat = "1..=2")]
        VariantA(u8),
        #[deku(id_pat = "_")]
        VariantB,
    }

    let input = [0x01, 0x02];
    let (_, v) = DekuTest::from_reader((&mut input.as_slice(), 0)).unwrap();
    assert_eq!(
        v,
        DekuTest {
            my_id: 0x01,
            enum_from_id: MyEnum::VariantA(2)
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());

    let input = [0x05];
    let (_, v) = DekuTest::from_reader((&mut input.as_slice(), 0)).unwrap();
    assert_eq!(
        v,
        DekuTest {
            my_id: 0x05,
            enum_from_id: MyEnum::VariantB
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());
}
