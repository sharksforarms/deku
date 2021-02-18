use deku::prelude::*;

#[test]
fn test_regular() {
    #[derive(Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    enum Request1 {
        #[deku(id = "0x01")]
        Cats { toy: u8 },

        #[deku(id = "0x10")]
        Dogs { ball: u8 },
    }

    assert_eq!(0x01, Request1::Cats { toy: 0 }.deku_id());
    assert_eq!(0x10, Request1::Dogs { ball: 0 }.deku_id());
}

#[test]
fn test_custom_type() {
    #[derive(Debug, DekuRead, PartialEq, DekuWrite)]
    #[deku(type = "u8")]
    enum Request2 {
        #[deku(id = "0x01")]
        Cats,

        #[deku(id = "0x10")]
        Dogs,
    }

    #[derive(Debug, DekuRead, DekuWrite)]
    #[deku(type = "Request2")]
    enum Request3 {
        #[deku(id = "Request2::Cats")]
        Cats,

        #[deku(id = "Request2::Dogs")]
        Dogs,
    }

    assert_eq!(Request2::Cats, Request3::Cats.deku_id());
    assert_eq!(Request2::Dogs, Request3::Dogs.deku_id());
}

#[test]
fn test_ctx() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct StructEnumId {
        my_id: u8,
        data: u8,
        #[deku(ctx = "*my_id")]
        enum_from_id: EnumId,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "my_id: u8", id = "my_id")]
    enum EnumId {
        #[deku(id = "1")]
        VarA(u8),
        #[deku(id = "2")]
        VarB,
    }

    assert_eq!(1, EnumId::VarA(0).deku_id());

    #[derive(Copy, Clone, PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8")]
    enum Nice {
        True = 0x00,
        False = 0x01,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    struct StructEnumId2 {
        my_id: Nice,
        data: u8,
        #[deku(ctx = "*my_id")]
        enum_from_id: EnumId2,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(ctx = "my_id: Nice", id = "my_id")]
    enum EnumId2 {
        #[deku(id = "Nice::True")]
        VarA(u8),
        #[deku(id = "Nice::False")]
        VarB,
    }

    assert_eq!(Nice::True, EnumId2::VarA(0).deku_id());
    assert_eq!(Nice::False, EnumId2::VarB.deku_id());
}

#[test]
fn test_ctx_and_type() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "u8", ctx = "_a: u8, _b: u8")]
    enum TopLevelCtxEnum {
        #[deku(id = "1")]
        VariantA(u8),
    }

    assert_eq!(1, TopLevelCtxEnum::VariantA(0).deku_id());
}

#[test]
fn test_advanced() {
    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(type = "[u8; 3]")]
    enum TestEnumArray {
        #[deku(id = b"123")]
        VarA,
        #[deku(id = "[1,1,1]")]
        VarB,
    }

    assert_eq!(b"123", &TestEnumArray::VarA.deku_id());
}
