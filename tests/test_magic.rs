use deku::prelude::*;
use hexlit::hex;
use rstest::rstest;
use std::convert::TryFrom;

pub mod samples {
    use super::*;

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(magic = b"deku")]
    pub struct MagicDeku {}

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(magic = b"deku", type = "u8")]
    pub enum EnumMagicDeku {
        #[deku(id = "0")]
        Variant,
    }

    #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
    #[deku(magic = b"UKED")]
    pub struct NestedMagicDeku {
        pub nested: MagicDeku
    }
}

#[rstest(input,
    case(&hex!("64656b75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64656bde")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("6465ad75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64be6b75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("ef656b75")),

    #[should_panic(expected = "Parse(\"not enough data: expected 8 bits got 0 bits\")")]
    case(&hex!("64656b")),
)]
fn test_magic_struct(input: &[u8]) {
    let ret_read = samples::MagicDeku::try_from(input).unwrap();

    assert_eq!(
        samples::MagicDeku{},
        ret_read
    )
}

#[rstest(input,
    case(&hex!("64656b7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64656bde00")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("6465ad7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64be6b7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("ef656b7500")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("64656b00")),

    #[should_panic(expected = "Parse(\"not enough data: expected 8 bits got 0 bits\")")]
    case(&hex!("64656b")),
)]
fn test_magic_enum(input: &[u8]) {
    let ret_read = samples::EnumMagicDeku::try_from(input).unwrap();

    assert_eq!(
        samples::EnumMagicDeku::Variant,
        ret_read
    )
}

#[rstest(input,
    case(&hex!("554b454464656b75")),

    #[should_panic(expected = "Parse(\"Missing magic value [100, 101, 107, 117]\")")]
    case(&hex!("554b4544deadbeef")),

    #[should_panic(expected = "Parse(\"Missing magic value [85, 75, 69, 68]\")")]
    case(&hex!("deadbeef64656b75")),
)]
fn test_nested_magic_struct(input: &[u8]) {
    let ret_read = samples::NestedMagicDeku::try_from(input).unwrap();

    assert_eq!(
        samples::NestedMagicDeku{nested: samples::MagicDeku{}},
        ret_read
    )
}
