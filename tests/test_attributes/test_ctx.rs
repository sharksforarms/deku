use std::convert::{TryFrom, TryInto};

use bitvec::bitvec;
use deku::bitvec::{BitView, Msb0};
use deku::prelude::*;

/// General smoke tests for ctx
/// TODO: These should be divided into smaller units

#[test]
fn test_ctx_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "a: u8, b: u8")]
    struct SubTypeNeedCtx {
        #[deku(
            reader = "(|rest|{u8::read(rest,()).map(|(slice,c)|(slice,(a+b+c) as usize))})(deku::rest)",
            writer = "(|c|{u8::write(&(c-a-b), deku::output, ())})(self.i as u8)"
        )]
        i: usize,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct FieldLevelCtxStruct {
        a: u8,
        b: u8,
        #[deku(ctx = "a + 1, *b")]
        c: SubTypeNeedCtx,
    }

    let test_data = [0x01_u8, 0x02, 0x03];

    let ret_read = FieldLevelCtxStruct::try_from(&test_data[..]).unwrap();
    assert_eq!(
        ret_read,
        FieldLevelCtxStruct {
            a: 0x01,
            b: 0x02,
            c: SubTypeNeedCtx {
                i: 0x01 + 1 + 0x02 + 0x03
            } // (a + 1) + b + c
        }
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data)
}

#[test]
fn test_top_level_ctx_enum() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8", ctx = "a: u8, b: u8")]
    enum TopLevelCtxEnum {
        #[deku(id = "1")]
        VariantA(
            #[deku(
                reader = "(|rest|{u8::read(rest,()).map(|(slice,c)|(slice,(a+b+c)))})(deku::rest)",
                writer = "(|c|{u8::write(&(c-a-b), deku::output, ())})(field_0)"
            )]
            u8,
        ),
    }

    let test_data = [0x01_u8, 0x03];
    let bit_slice = test_data.view_bits();
    let (amt_read, ret_read) = TopLevelCtxEnum::read(bit_slice, (1, 2)).unwrap();
    assert!(bit_slice[amt_read..].is_empty());
    assert_eq!(ret_read, TopLevelCtxEnum::VariantA(0x06));

    let mut ret_write = bitvec![u8, Msb0;];
    ret_read.write(&mut ret_write, (1, 2)).unwrap();
    assert_eq!(ret_write.into_vec(), &test_data[..]);
}

#[test]
fn test_top_level_ctx_enum_default() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8", ctx = "a: u8, b: u8", ctx_default = "1,2")]
    enum TopLevelCtxEnumDefault {
        #[deku(id = "1")]
        VariantA(
            #[deku(
                reader = "(|rest|{u8::read(rest,()).map(|(slice,c)|(slice,(a+b+c)))})(deku::rest)",
                writer = "(|c|{u8::write(&(c-a-b), deku::output, ())})(field_0)"
            )]
            u8,
        ),
    }

    let expected = TopLevelCtxEnumDefault::VariantA(0x06);
    let test_data = [0x01_u8, 0x03];

    // Use default
    let ret_read = TopLevelCtxEnumDefault::try_from(test_data.as_ref()).unwrap();
    assert_eq!(expected, ret_read);
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(test_data.to_vec(), ret_write);

    // Use context
    let bit_slice = test_data.view_bits();
    let (amt_read, ret_read) = TopLevelCtxEnumDefault::read(bit_slice, (1, 2)).unwrap();
    assert!(bit_slice[amt_read..].is_empty());
    assert_eq!(ret_read, TopLevelCtxEnumDefault::VariantA(0x06));
    let mut ret_write = bitvec![u8, Msb0;];
    ret_read.write(&mut ret_write, (1, 2)).unwrap();
    assert_eq!(test_data.to_vec(), ret_write.into_vec());
}

#[test]
fn test_struct_enum_ctx_id() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "my_id: u8, data: usize", id = "my_id, data")]
    enum EnumId {
        #[deku(id = "(1, 1)")]
        VarA(u8),
        #[deku(id = "(2, 2)")]
        VarB,
        #[deku(id = "(2, 3)")]
        VarC(u8),
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "my_id: u8", id = "my_id")]
    enum EnumJustId {
        #[deku(id = "1")]
        VarA(u8),
        #[deku(id = "2")]
        VarB,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct StructEnumId {
        my_id: u8,
        #[deku(bytes = "1")]
        data: usize,
        #[deku(ctx = "*my_id, *data")]
        enum_from_id: EnumId,
        #[deku(ctx = "*my_id")]
        enum_from_just_id: EnumJustId,
    }

    // VarA
    let test_data = [0x01_u8, 0x01, 0xab, 0xab];
    let ret_read = StructEnumId::try_from(test_data.as_ref()).unwrap();

    assert_eq!(
        StructEnumId {
            my_id: 0x01,
            data: 0x01,
            enum_from_id: EnumId::VarA(0xab),
            enum_from_just_id: EnumJustId::VarA(0xab),
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);

    // VarB
    let test_data = [0x02_u8, 0x02];
    let ret_read = StructEnumId::try_from(test_data.as_ref()).unwrap();

    assert_eq!(
        StructEnumId {
            my_id: 0x02,
            data: 0x02,
            enum_from_id: EnumId::VarB,
            enum_from_just_id: EnumJustId::VarB,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);

    // VarC
    let test_data = [0x02_u8, 0x03, 0xcc];
    let (_, ret_read) = StructEnumId::from_bytes((test_data.as_ref(), 0)).unwrap();

    assert_eq!(
        StructEnumId {
            my_id: 0x02,
            data: 0x03,
            enum_from_id: EnumId::VarC(0xcc),
            enum_from_just_id: EnumJustId::VarB,
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);
}

#[test]
fn test_ctx_default_struct() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "a: u8, b: u8", ctx_default = "1, 2")]
    struct TopLevelCtxStructDefault {
        #[deku(cond = "a == 1")]
        a: Option<u8>,
        #[deku(cond = "b == 1")]
        b: Option<u8>,
    }

    let expected = TopLevelCtxStructDefault {
        a: Some(0xff),
        b: None,
    };

    let test_data = [0xffu8];

    // Use default
    let ret_read = TopLevelCtxStructDefault::try_from(test_data.as_ref()).unwrap();
    assert_eq!(expected, ret_read);
    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data);

    // Use context
    let bit_slice = test_data.view_bits();
    let (amt_read, ret_read) = TopLevelCtxStructDefault::read(bit_slice, (1, 2)).unwrap();
    assert_eq!(bit_slice.len(), amt_read);
    assert_eq!(expected, ret_read);
    let mut ret_write = bitvec![u8, Msb0;];
    ret_read.write(&mut ret_write, (1, 2)).unwrap();
    assert_eq!(test_data.to_vec(), ret_write.into_vec());
}

#[test]
fn test_enum_endian_ctx() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u32", endian = "endian", ctx = "endian: deku::ctx::Endian")]
    enum EnumTypeEndianCtx {
        #[deku(id = "0xDEADBEEF")]
        VarA(u8),
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct EnumTypeEndian {
        #[deku(endian = "big")]
        t: EnumTypeEndianCtx,
    }

    let test_data = [0xdeu8, 0xad, 0xbe, 0xef, 0xff];
    let ret_read = EnumTypeEndian::try_from(test_data.as_ref()).unwrap();

    assert_eq!(
        EnumTypeEndian {
            t: EnumTypeEndianCtx::VarA(0xff)
        },
        ret_read
    );

    let ret_write: Vec<u8> = ret_read.try_into().unwrap();
    assert_eq!(ret_write, test_data)
}
