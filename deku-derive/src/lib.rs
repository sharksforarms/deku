use darling::ast;
use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;
use proc_macro2::TokenStream;

mod macros;
use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};

#[derive(Debug, Clone, Copy, PartialEq, FromMeta)]
#[darling(default)]
pub (crate) enum EndianNess {
    Little,
    Big,
}

impl Default for EndianNess {
    fn default() -> Self {
        #[cfg(target_endian = "little")]
        let ret = EndianNess::Little;

        #[cfg(target_endian = "big")]
        let ret = EndianNess::Big;

        ret
    }
}

#[derive(Debug, FromDeriveInput)]
// Process all `deku` attributes and only support structs
#[darling(attributes(deku), supports(struct_any))]
struct DekuReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), DekuFieldReceiver>,

    // Default EndianNess
    #[darling(default)]
    endian: EndianNess,
}

impl DekuReceiver {
    fn emit_reader(&self) -> Result<TokenStream, darling::Error> {
        emit_deku_read(self)
    }

    fn emit_writer(&self) -> Result<TokenStream, darling::Error> {
        emit_deku_write(self)
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(deku))]
struct DekuFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    endian: Option<EndianNess>,

    #[darling(default)]
    bits: Option<usize>,

    #[darling(default)]
    len: Option<String>,

    #[darling(default)]
    bytes: Option<usize>,

    #[darling(default)]
    reader: Option<String>,

    #[darling(default)]
    writer: Option<String>,
}

#[proc_macro_derive(DekuRead, attributes(deku))]
pub fn proc_deku_read(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let receiver = DekuReceiver::from_derive_input(&syn::parse(input).unwrap()).unwrap();
    let tokens = receiver.emit_reader().unwrap();
    tokens.into()
}

#[proc_macro_derive(DekuWrite, attributes(deku))]
pub fn proc_deku_write(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let receiver = DekuReceiver::from_derive_input(&syn::parse(input).unwrap()).unwrap();
    let tokens = receiver.emit_writer().unwrap();
    tokens.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use syn::parse_str;

    #[rstest(input,
        // Valid
        case::struct_unnamed(r#"struct Test(u8, u8);"#),
        case::struct_unnamed_attrs(r#"struct Test(#[deku(bits=4)] u8, u8);"#),
        case::struct_all_attrs(r#"
        struct Test {
            #[deku(bits = 4)]
            field_a: u8,
            #[deku(bytes = 4)]
            field_b: u64,
            #[deku(endian = little)]
            field_c: u32,
            #[deku(endian = big)]
            field_d: u32,
        }"#),

        // Invalid
        #[should_panic(expected = "UnsupportedShape(\"enum\")")]
        case::invalid_enum(r#"enum Test { A }"#),
        #[should_panic(expected = "UnknownField(ErrorUnknownField { name: \"sbits\", did_you_mean: Some(\"bits\") })")]
        case::invalid_field(r#"struct Test(#[deku(sbits=4)] u8);"#),
        #[should_panic(expected = "DuplicateField(\"bits\")")]
        case::invalid_field_duplicate(r#"struct Test(#[deku(bits=4, bits=5)] u8);"#),
        #[should_panic(expected = "DuplicateField(\"both \\\"bits\\\" and \\\"bytes\\\" specified\")")]
        case::invalid_field_bitsnbytes(r#"struct Test(#[deku(bits=4, bytes=1)] u8);"#),

        // TODO: these tests should error/warn eventually
        // error: trying to store 9 bits in 8 bit type
        case::invalid_storage(r#"struct Test(#[deku(bits=9)] u8);"#),
        // warn: trying to set endian on a type which wouldn't make a difference
        case::invalid_endian(r#"struct Test(#[endian=big] u8);"#),
    )]
    fn test_macro(input: &str) {
        let parsed = parse_str(input).unwrap();

        let receiver = DekuReceiver::from_derive_input(&parsed).unwrap();
        let res_reader = receiver.emit_reader();
        let res_writer = receiver.emit_writer();

        res_reader.unwrap();
        res_writer.unwrap();
    }
}
