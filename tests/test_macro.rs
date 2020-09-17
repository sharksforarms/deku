use deku::prelude::*;
use hexlit::hex;
use rstest::rstest;
use std::convert::{TryFrom, TryInto};

pub mod samples {
    use super::*;

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct DoubleNestedDeku {
        pub data: u16,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct NestedDeku {
        #[deku(bits = "6")]
        pub nest_a: u8,
        #[deku(bits = "2")]
        pub nest_b: u8,

        pub inner: DoubleNestedDeku,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct UnNamedDeku(
        pub u8,
        #[deku(bits = "2")] pub u8,
        #[deku(bits = "6")] pub u8,
        #[deku(bytes = "2")] pub u16,
        #[deku(endian = "big")] pub u16,
        pub NestedDeku,
        #[deku(update = "self.7.len()")] pub u8,
        #[deku(count = "field_6")] pub Vec<u8>,
    );

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct NamedDeku {
        pub field_a: u8,
        #[deku(bits = "2")]
        pub field_b: u8,
        #[deku(bits = "6")]
        pub field_c: u8,
        #[deku(bytes = "2")]
        pub field_d: u16,
        #[deku(endian = "big")]
        pub field_e: u16,
        pub field_f: NestedDeku,
        pub vec_len: u8,
        #[deku(count = "vec_len")]
        pub vec_data: Vec<u8>,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum EnumDeku {
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
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum EnumDekuDefault {
        #[deku(id = "1")]
        VarA(u8),

        VarDefault(u8, u8),
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct VecCountDeku {
        #[deku(update = "self.vec_data.len()")]
        pub count: u8,
        #[deku(count = "count")]
        pub vec_data: Vec<u8>,
    }

    #[derive(PartialEq, Debug, DekuRead)]
    pub struct MapDeku {
        #[deku(map = "|field: u8| -> Result<_, DekuError> { Ok(field.to_string()) }")]
        pub field_a: String,
        #[deku(map = "MapDeku::map_field_b")]
        pub field_b: String,
    }

    impl MapDeku {
        fn map_field_b(field_b: u8) -> Result<String, DekuError> {
            Ok(field_b.to_string())
        }
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct GenericStructDeku<T: deku::DekuWrite + deku::DekuRead>
    where
        T: deku::DekuWrite + deku::DekuRead,
    {
        pub field_a: T,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum GenericEnumDeku<T: deku::DekuRead + deku::DekuWrite>
    where
        T: deku::DekuWrite + deku::DekuRead,
    {
        #[deku(id = "1")]
        VariantT(T),
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct SkipDeku {
        pub field_a: u8,
        #[deku(skip)]
        pub field_b: Option<u8>,
        pub field_c: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct DefaultDeku {
        pub field_a: u8,
        #[deku(skip, default = "5")]
        pub field_b: u8,
        pub field_c: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct SkipCondDeku {
        pub field_a: u8,
        #[deku(skip, cond = "*field_a == 0x01", default = "5")]
        pub field_b: u8,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct CondDeku {
        pub field_a: u8,
        #[deku(cond = "*field_a == 0x01")]
        pub field_b: Option<u8>,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "_a: u8, _b: u8")]
    pub struct TopLevelCtxStruct {}

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "a: u8, b: u8")]
    pub struct SubTypeNeedCtx {
        #[deku(
            reader = "(|rest|{u8::read(rest,()).map(|(slice,c)|(slice,(a+b+c) as usize))})(rest)",
            writer = "(|c|{u8::write(&(c-a-b), ())})(self.i as u8)"
        )]
        pub(crate) i: usize,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "a: u8, b: u8", ctx_default = "1, 2")]
    pub struct TopLevelCtxStructDefault {
        #[deku(cond = "a == 1")]
        pub a: Option<u8>,
        #[deku(cond = "b == 1")]
        pub b: Option<u8>,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct FieldLevelCtxStruct {
        pub a: u8,
        pub b: u8,
        #[deku(ctx = "a + 1, *b")]
        pub c: SubTypeNeedCtx,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8", ctx = "a: u8, b: u8")]
    pub enum TopLevelCtxEnum {
        #[deku(id = "1")]
        VariantA(
            #[deku(
                reader = "(|rest|{u8::read(rest,()).map(|(slice,c)|(slice,(a+b+c)))})(rest)",
                writer = "(|c|{u8::write(&(c-a-b), ())})(field_0)"
            )]
            u8,
        ),
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8", ctx = "a: u8, b: u8", ctx_default = "1,2")]
    pub enum TopLevelCtxEnumDefault {
        #[deku(id = "1")]
        VariantA(
            #[deku(
                reader = "(|rest|{u8::read(rest,()).map(|(slice,c)|(slice,(a+b+c)))})(rest)",
                writer = "(|c|{u8::write(&(c-a-b), ())})(field_0)"
            )]
            u8,
        ),
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    pub enum VariantLevelCtxEnum {
        #[deku(id = "1")]
        VariantA {
            a: u8,
            b: u8,
            #[deku(ctx = "*a, *b")]
            c: SubTypeNeedCtx,
        },
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct StructEnumId {
        pub my_id: u8,
        pub data: u8,
        #[deku(ctx = "*my_id")]
        pub enum_from_id: EnumId,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "my_id: u8", id = "my_id")]
    pub enum EnumId {
        #[deku(id = "1")]
        VarA(u8),
        #[deku(id = "2")]
        VarB,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    pub struct EnumTypeEndian {
        #[deku(endian = "big")]
        pub t: EnumTypeEndianCtx,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u32", endian = "endian", ctx = "endian: deku::ctx::Endian")]
    pub enum EnumTypeEndianCtx {
        #[deku(id = "0xDEADBEEF")]
        VarA(u8),
    }
}

#[test]
#[should_panic(expected = r#"Parse("Too much data")"#)]
fn test_too_much_data() {
    let test_data = [0u8; 100].as_ref();
    samples::UnNamedDeku::try_from(test_data).unwrap();
}

#[test]
fn test_unnamed_struct() {
    let test_data: Vec<u8> = [
        0xFF,
        0b1001_0110,
        0xAA,
        0xBB,
        0xCC,
        0xDD,
        0b1001_0110,
        0xCC,
        0xDD,
        0x02,
        0xBE,
        0xEF,
    ]
    .to_vec();

    // Read
    let ret_read = samples::UnNamedDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::UnNamedDeku(
            0xFF,
            0b0000_0010,
            0b0001_0110,
            0xBBAA,
            0xCCDD,
            samples::NestedDeku {
                nest_a: 0b00_100101,
                nest_b: 0b10,
                inner: samples::DoubleNestedDeku { data: 0xDDCC }
            },
            0x02,
            vec![0xBE, 0xEF],
        ),
        ret_read
    );

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_named_struct() {
    let test_data: Vec<u8> = [
        0xFF,
        0b1001_0110,
        0xAA,
        0xBB,
        0xCC,
        0xDD,
        0b1001_0110,
        0xCC,
        0xDD,
        0x02,
        0xBE,
        0xEF,
    ]
    .to_vec();

    // Read
    let ret_read = samples::NamedDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::NamedDeku {
            field_a: 0xFF,
            field_b: 0b0000_0010,
            field_c: 0b0001_0110,
            field_d: 0xBBAA,
            field_e: 0xCCDD,
            field_f: samples::NestedDeku {
                nest_a: 0b00_100101,
                nest_b: 0b10,
                inner: samples::DoubleNestedDeku { data: 0xDDCC }
            },
            vec_len: 0x02,
            vec_data: vec![0xBE, 0xEF]
        },
        ret_read
    );

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[rstest(input,expected,
    case(&hex!("01AB"), samples::EnumDeku::VarA(0xAB)),
    case(&hex!("0269"), samples::EnumDeku::VarB(0b0110, 0b1001)),
    case(&hex!("0302AABB"), samples::EnumDeku::VarC{field_a: 0x02, field_b: vec![0xAA, 0xBB]}),
    case(&hex!("0402AABB"), samples::EnumDeku::VarD(0x02, vec![0xAA, 0xBB])),

    #[should_panic(expected = "Parse(\"Could not match enum variant id = 255 on enum `EnumDeku`\")")]
    case(&hex!("FFAB"), samples::EnumDeku::VarA(0xFF))
)]
fn test_enum(input: &[u8], expected: samples::EnumDeku) {
    let ret_read = samples::EnumDeku::try_from(input).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);
}

#[rstest(input,expected,
    case(&hex!("01AB"), samples::EnumDekuDefault::VarA(0xAB)),
    case(&hex!("FFAB"), samples::EnumDekuDefault::VarDefault(0xFF, 0xAB)),

    #[should_panic(expected = "Parse(\"Too much data\")")]
    case(&hex!("FFFFFF"), samples::EnumDekuDefault::VarA(0xAB)),
)]
fn test_enum_default(input: &[u8], expected: samples::EnumDekuDefault) {
    let ret_read = samples::EnumDekuDefault::try_from(input).unwrap();
    assert_eq!(expected, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(input.to_vec(), ret_write);
}

#[test]
fn test_dynamic_vec_count() {
    let test_data: Vec<u8> = [0x02, 0xAA, 0xBB].to_vec();

    // Read
    let mut ret_read = samples::VecCountDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::VecCountDeku {
            count: 0x02,
            vec_data: vec![0xAA, 0xBB]
        },
        ret_read
    );

    // Add an item to the vec
    ret_read.vec_data.push(0xFF);
    ret_read.update().unwrap();

    // Write
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!([0x03, 0xAA, 0xBB, 0xFF].to_vec(), ret_write);
}

#[test]
#[should_panic(
    expected = "Parse(\"error parsing int: out of range integral type conversion attempted\")"
)]
fn test_dynamic_vec_count_error() {
    let mut val = samples::VecCountDeku {
        count: 0x02,
        vec_data: vec![0xAA, 0xBB],
    };

    // `count` is a u8, add u8::MAX ++ items and try to update
    for _ in 0..std::u8::MAX {
        val.vec_data.push(0xFF);
    }
    val.update().unwrap();
}

#[test]
fn test_map() {
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = samples::MapDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::MapDeku {
            field_a: "1".to_string(),
            field_b: "2".to_string(),
        },
        ret_read
    );
}

#[test]
fn test_generic_struct_deku() {
    let test_data: Vec<u8> = [0x01].to_vec();

    let ret_read = samples::GenericStructDeku::<u8>::try_from(test_data.as_ref()).unwrap();
    assert_eq!(samples::GenericStructDeku::<u8> { field_a: 0x01 }, ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_generic_enum_deku() {
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = samples::GenericEnumDeku::<u8>::try_from(test_data.as_ref()).unwrap();
    assert_eq!(samples::GenericEnumDeku::<u8>::VariantT(0x02), ret_read);

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_skip_deku() {
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = samples::SkipDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::SkipDeku {
            field_a: 0x01,
            field_b: None,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_default_deku() {
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = samples::DefaultDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::DefaultDeku {
            field_a: 0x01,
            field_b: 0x05,
            field_c: 0x02,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_skip_cond_deku() {
    let test_data: Vec<u8> = [0x01].to_vec();

    let ret_read = samples::SkipCondDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::SkipCondDeku {
            field_a: 0x01,
            field_b: 0x05, // default
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(vec![0x01], ret_write);
}

#[test]
fn test_cond_deku() {
    let test_data: Vec<u8> = [0x01, 0x02].to_vec();

    let ret_read = samples::CondDeku::try_from(test_data.as_ref()).unwrap();
    assert_eq!(
        samples::CondDeku {
            field_a: 0x01,
            field_b: Some(0x02),
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data, ret_write);
}

#[test]
fn test_ctx_struct() {
    let test_data = [0x01_u8, 0x02, 0x03];

    let ret_read = samples::FieldLevelCtxStruct::try_from(&test_data[..]).unwrap();
    assert_eq!(
        ret_read,
        samples::FieldLevelCtxStruct {
            a: 0x01,
            b: 0x02,
            c: samples::SubTypeNeedCtx {
                i: 0x01 + 1 + 0x02 + 0x03
            } // (a + 1) + b + c
        }
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data)
}

#[test]
fn test_top_level_ctx_enum() {
    let test_data = [0x01_u8, 0x03];
    let (rest, ret_read) = samples::TopLevelCtxEnum::read(test_data.view_bits(), (1, 2)).unwrap();
    assert!(rest.is_empty());
    assert_eq!(ret_read, samples::TopLevelCtxEnum::VariantA(0x06));

    let ret_write = ret_read.write((1, 2)).unwrap();
    assert_eq!(ret_write.into_vec(), &test_data[..]);
}

#[test]
fn test_top_level_ctx_enum_default() {
    let expected = samples::TopLevelCtxEnumDefault::VariantA(0x06);
    let test_data = [0x01_u8, 0x03];

    // Use default
    let ret_read = samples::TopLevelCtxEnumDefault::try_from(test_data.as_ref()).unwrap();
    assert_eq!(expected, ret_read);
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data.to_vec(), ret_write);

    // Use context
    let (rest, ret_read) =
        samples::TopLevelCtxEnumDefault::read(test_data.view_bits(), (1, 2)).unwrap();
    assert!(rest.is_empty());
    assert_eq!(ret_read, samples::TopLevelCtxEnumDefault::VariantA(0x06));
    let ret_write = ret_read.write((1, 2)).unwrap();
    assert_eq!(test_data.to_vec(), ret_write.into_vec());
}

#[test]
fn test_struct_enum_ctx_id() {
    let test_data = [0x01_u8, 0xff, 0xab];
    let ret_read = samples::StructEnumId::try_from(test_data.as_ref()).unwrap();

    assert_eq!(
        samples::StructEnumId {
            my_id: 0x01,
            data: 0xff,
            enum_from_id: samples::EnumId::VarA(0xab),
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data)
}

#[test]
fn test_ctx_default_struct() {
    let expected = samples::TopLevelCtxStructDefault {
        a: Some(0xff),
        b: None,
    };

    let test_data = [0xffu8];

    // Use default
    let ret_read = samples::TopLevelCtxStructDefault::try_from(test_data.as_ref()).unwrap();
    assert_eq!(expected, ret_read);
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);

    // Use context
    let (rest, ret_read) =
        samples::TopLevelCtxStructDefault::read(test_data.view_bits(), (1, 2)).unwrap();
    assert!(rest.is_empty());
    assert_eq!(expected, ret_read);
    let ret_write = ret_read.write((1, 2)).unwrap();
    assert_eq!(test_data.to_vec(), ret_write.into_vec());
}

#[test]
fn test_enum_endian_ctx() {
    let test_data = [0xdeu8, 0xad, 0xbe, 0xef, 0xff];
    let ret_read = samples::EnumTypeEndian::try_from(test_data.as_ref()).unwrap();

    assert_eq!(
        samples::EnumTypeEndian {
            t: samples::EnumTypeEndianCtx::VarA(0xFF)
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data)
}

#[test]
fn test_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macro_read/*.rs");
}
