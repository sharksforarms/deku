#![cfg(feature = "alloc")]

//! General smoke tests for enums

// TODO: These should be divided into smaller tests

use core::convert::TryFrom;
use no_std_io::io::Cursor;

use deku::prelude::*;

#[allow(unused_imports)]
use hexlit::hex;
#[allow(unused_imports)]
use rstest::*;

#[cfg(feature = "bits")]
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

#[cfg(all(feature = "bits", feature = "descriptive-errors"))]
#[rstest(input,expected,
    case(&hex!("01AB"), TestEnum::VarA(0xAB)),
    case(&hex!("0269"), TestEnum::VarB(0b0110, 0b1001)),
    case(&hex!("0302AABB"), TestEnum::VarC{field_a: 0x02, field_b: vec![0xAA, 0xBB]}),
    case(&hex!("0402AABB"), TestEnum::VarD(0x02, vec![0xAA, 0xBB])),
    case(&hex!("FF01"), TestEnum::VarDefault{id: 0xFF, value: 0x01}),

    #[should_panic(expected = "Parse(\"Too much data: Read 2 but total length was 3\")")]
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
#[should_panic(expected = "Could not match enum variant")]
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
#[repr(u8)]
#[deku(id_type = "u8")]
#[cfg(all(feature = "alloc", feature = "std"))]
enum TestEnumDiscriminant {
    VarA = 0x00,
    VarB,
    VarC = 0x02,
}

#[rstest(input, expected,
    case(&hex!("00"), TestEnumDiscriminant::VarA),
    case(&hex!("01"), TestEnumDiscriminant::VarB),
    case(&hex!("02"), TestEnumDiscriminant::VarC),

    #[should_panic(expected = "Could not match enum variant")]
    case(&hex!("03"), TestEnumDiscriminant::VarA),
)]
// TODO: Switch std::convert::TryInto to core::convert::TryInto
#[cfg(all(feature = "alloc", feature = "std"))]
fn test_enum_discriminant(input: &[u8], expected: TestEnumDiscriminant) {
    let input = input.to_vec();
    let ret_read = TestEnumDiscriminant::try_from(input.as_slice()).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input, ret_write);
}

#[test]
#[cfg(all(feature = "alloc", feature = "std"))]
fn test_enum_array_type() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(id_type = "[u8; 3]")]
    enum TestEnumArray {
        #[deku(id = b"123")]
        VarA,
        #[deku(id = "[1,1,1]")]
        VarB,
        #[deku(id_pat = "_")]
        VarC([u8; 3]),
    }

    let input = b"123".to_vec();

    let ret_read = TestEnumArray::try_from(input.as_slice()).unwrap();
    assert_eq!(TestEnumArray::VarA, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);

    let input = b"321".to_vec();

    let ret_read = TestEnumArray::try_from(input.as_slice()).unwrap();
    assert_eq!(TestEnumArray::VarC([b"3"[0], b"2"[0], b"1"[0]]), ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);
}

#[cfg(feature = "bits")]
#[test]
fn test_enum_id_pat_with_discriminant() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct DekuTest {
        inner: TestEnum,
        #[deku(bits = 5)]
        rest: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = "3")]
    #[repr(u8)]
    enum TestEnum {
        VarA = 0,
        VarB,
        #[deku(id_pat = "_")]
        VarC,
    }

    let input = &[0b001_00101];
    let ret_read = DekuTest::try_from(input.as_slice()).unwrap();
    assert_eq!(
        DekuTest {
            inner: TestEnum::VarB,
            rest: 0b00101
        },
        ret_read
    );
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);

    let input = &[0b101_00101];
    let ret_read = DekuTest::try_from(input.as_slice()).unwrap();
    assert_eq!(
        DekuTest {
            inner: TestEnum::VarC,
            rest: 0b00101
        },
        ret_read
    );
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0b010_00101], ret_write);
}

#[cfg(feature = "bits")]
#[test]
fn test_enum_id_pat_with_discriminant_and_storage() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct DekuTest {
        inner: TestEnumStorage,
        #[deku(bits = 5)]
        rest: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = "3")]
    #[repr(u8)]
    enum TestEnumStorage {
        VarA = 0,
        VarB,
        #[deku(id_pat = "_")]
        VarC(u8),
    }

    let input = &[0b001_00101];
    let ret_read = DekuTest::try_from(input.as_slice()).unwrap();
    assert_eq!(
        DekuTest {
            inner: TestEnumStorage::VarB,
            rest: 0b00101
        },
        ret_read
    );
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);

    let input = &[0b101_00101];
    let ret_read = DekuTest::try_from(input.as_slice()).unwrap();
    assert_eq!(
        DekuTest {
            inner: TestEnumStorage::VarC(0b101),
            rest: 0b00101
        },
        ret_read
    );
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);
}

#[test]
#[cfg(all(feature = "alloc", feature = "std"))]
fn test_id_pat_with_id() {
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

        // Id type can differ from id_type
        #[deku(id_pat = "4..=5")]
        VariantDiffType(u16),

        #[deku(id_pat = "_")]
        VariantB,
    }

    let input = [0x01, 0x02];
    let mut cursor = Cursor::new(input);
    let (_, v) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(
        v,
        DekuTest {
            my_id: 0x01,
            enum_from_id: MyEnum::VariantA(2)
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());

    let input = [0x04, 0x02, 0xff];
    let mut cursor = Cursor::new(input);
    let (_, v) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(
        v,
        DekuTest {
            my_id: 0x04,
            enum_from_id: MyEnum::VariantDiffType(0xff02)
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());

    let input = [0x06];
    let mut cursor = Cursor::new(input);
    let (_, v) = DekuTest::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(
        v,
        DekuTest {
            my_id: 0x06,
            enum_from_id: MyEnum::VariantB,
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());
}

#[test]
#[cfg(feature = "bits")]
fn id_pat_with_id_bits() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(id_type = "u8", bits = "2")]
    pub enum IdPatBits {
        #[deku(id = 1)]
        A(#[deku(bits = 6)] u8),

        #[deku(id_pat = "_")]
        B(u8, #[deku(bits = 6)] u8),
    }

    let input = [0b1100_1111];
    let mut cursor = Cursor::new(input);
    let (_, v) = IdPatBits::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(v, IdPatBits::B(0b11, 0b00_1111));
    assert_eq!(input, &*v.to_bytes().unwrap());
}

#[test]
#[cfg(feature = "bits")]
fn test_litbool_as_id() {
    use deku::prelude::*;

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq)]
    pub struct A {
        #[deku(bits = 1)]
        bit: bool,
        #[deku(ctx = "*bit")]
        var: Var,
    }

    #[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq)]
    #[deku(id = "bit", ctx = "bit: bool")]
    pub enum Var {
        #[deku(id = false)]
        False(#[deku(bits = 15)] u16),
        #[deku(id = true)]
        True(#[deku(bits = 15)] u16),
    }
    let input = [0b1000_0000, 0xff];
    let mut cursor = Cursor::new(input);
    let (_, v) = A::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(
        v,
        A {
            bit: true,
            var: Var::True(0x7f01),
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());
    let input = [0b0000_0000, 0xff];
    let mut cursor = Cursor::new(input);
    let (_, v) = A::from_reader((&mut cursor, 0)).unwrap();
    assert_eq!(
        v,
        A {
            bit: false,
            var: Var::False(0x7f01),
        }
    );
    assert_eq!(input, &*v.to_bytes().unwrap());
}

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u16", id_endian = "big", endian = "little")]
#[cfg(all(feature = "alloc", feature = "std"))]
enum VariableEndian {
    #[deku(id = "0x01")]
    Little(u16),
    #[deku(id = "0x02")]
    Big {
        #[deku(endian = "big")]
        x: u16,
    },
}

#[rstest(input, expected,
case(&hex!("00010100"), VariableEndian::Little(1)),
case(&hex!("00020100"), VariableEndian::Big{x: 256})
)]
#[cfg(all(feature = "alloc", feature = "std"))]
fn test_variable_endian_enum(input: &[u8], expected: VariableEndian) {
    let ret_read = VariableEndian::try_from(input).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);
}

#[test]
fn test_repr_assignment_with_id_via_ctx() {
    use deku::ctx::Endian;

    #[derive(Debug, DekuRead, DekuWrite, Eq, PartialEq)]
    #[deku(ctx = "endian: Endian, mid: u8", id = "mid", endian = "endian")]
    #[repr(u8)]
    enum Body {
        First = 0x00,
        #[deku(id = "0x01")]
        Second(u8),
    }

    #[derive(Debug, DekuRead, DekuWrite, Eq, PartialEq)]
    #[deku(endian = "little")]
    struct Message {
        id: u8,
        header: u16,
        #[deku(ctx = "*id")]
        body: Body,
    }

    let input = [0u8, 1u8, 0u8];
    let mut cursor = Cursor::new(input);
    assert_eq!(
        Message {
            id: 0,
            header: 1,
            body: Body::First,
        },
        Message::from_reader((&mut cursor, 0)).unwrap().1
    );

    let input = [1u8, 2u8, 0u8, 3u8];
    let mut cursor = Cursor::new(input);
    assert_eq!(
        Message {
            id: 1,
            header: 2,
            body: Body::Second(3),
        },
        Message::from_reader((&mut cursor, 0)).unwrap().1
    );
}
