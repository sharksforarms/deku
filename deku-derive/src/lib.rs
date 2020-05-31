use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
mod macros;
use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};

#[derive(Debug, Clone, Copy, PartialEq, FromMeta)]
#[darling(default)]
/// Endian types for `endian` attribute
pub(crate) enum EndianNess {
    Little,
    Big,
}

impl Default for EndianNess {
    fn default() -> Self {
        #[cfg(target_endian = "little")]
        let ret = EndianNess::Little;

        #[cfg(target_endian = "big")]
        let rets = EndianNess::Big;

        ret
    }
}

/// Receiver for the top-level struct or enum
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(deku),
    supports(struct_any, enum_any),
    map = "DekuReceiver::init"
)]
struct DekuReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<DekuVariantReceiver, DekuFieldReceiver>,

    /// Endian default for the fields
    #[darling(default)]
    endian: EndianNess,

    /// enum only: type of the enum `id`
    #[darling(default)]
    id_type: Option<syn::Ident>,

    // enum only: bit size of the enum `id`
    #[darling(default)]
    id_bits: Option<usize>,

    // enum only: byte size of the enum `id`
    #[darling(default)]
    id_bytes: Option<usize>,
}

impl DekuReceiver {
    /// Initialize and validate the DekuReceiver
    fn init(self) -> Self {
        // Validate id_* attributes are being used on an enum
        if (self.id_type.is_some() || self.id_bits.is_some() || self.id_bytes.is_some())
            && !self.data.is_enum()
        {
            panic!("`id_*` attributes only supported on enum")
        }

        // Validate that `id_type` is set with a size
        if (self.id_bits.is_some() || self.id_bytes.is_some()) && self.id_type.is_none() {
            panic!("`id_type` must be specified with `id_bits` or `id_bytes`");
        }

        // Validate either `id_bits` or `id_bytes` is specified
        if self.id_bits.is_some() && self.id_bytes.is_some() {
            panic!("conflicting: both \"id_bits\" and \"id_bytes\" specified on field");
        }

        // Calculate bit size from both attributes
        let id_bits = self.id_bits.or_else(|| self.id_bytes.map(|v| v * 8));
        let id_bytes = None;

        // Return updated receiver
        Self {
            id_bits,
            id_bytes,
            ..self
        }
    }

    fn emit_reader(&self) -> Result<TokenStream, darling::Error> {
        emit_deku_read(self)
    }

    fn emit_writer(&self) -> Result<TokenStream, darling::Error> {
        emit_deku_write(self)
    }
}

/// Parse a TokenStream from an Option<String>
fn option_as_tokenstream(input: Option<String>) -> Option<TokenStream> {
    input.map(|v| {
        v.parse::<TokenStream>()
            .expect("could not parse token stream")
    })
}

/// Generate field name which supports both un-named/named structs/enums
/// `ident` is Some if the container has named fields
/// `index` is the numerical index of the current field used in un-named containers
/// `prefix` is true in the case of variable declarations and match arms,
/// false when the raw field is required, for example a field access
fn gen_field_ident<T: ToString>(ident: Option<T>, index: usize, prefix: bool) -> TokenStream {
    let field_name = match ident {
        Some(field_name) => field_name.to_string(),
        None => {
            let index = syn::Index::from(index);
            let prefix = if prefix { "field_" } else { "" };
            format!("{}{}", prefix, quote! { #index })
        }
    };

    field_name.parse().unwrap()
}

/// Receiver for the field-level attributes inside a struct/enum variant
#[derive(Debug, FromField)]
#[darling(attributes(deku), map = "DekuFieldReceiver::init")]
struct DekuFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    /// Endianness for the field
    #[darling(default)]
    endian: Option<EndianNess>,

    /// field bit size
    #[darling(default)]
    bits: Option<usize>,

    /// field byte size
    #[darling(default)]
    bytes: Option<usize>,

    /// reference to another field providing the  length
    #[darling(default)]
    len: Option<String>,

    /// custom field reader code
    #[darling(default, map = "option_as_tokenstream")]
    reader: Option<TokenStream>,

    /// custom field writer code
    #[darling(default, map = "option_as_tokenstream")]
    writer: Option<TokenStream>,
}

impl DekuFieldReceiver {
    /// Initialize and validate the DekuFieldReceiver
    fn init(self) -> Self {
        // Validate either `bits` or `bytes` is specified
        if self.bits.is_some() && self.bytes.is_some() {
            panic!("conflicting: both \"bits\" and \"bytes\" specified on field");
        }

        // Calculate bit size from both attributes
        let bits = self.bits.or_else(|| self.bytes.map(|v| v * 8));
        let bytes = None;

        // Return updated receiver
        Self {
            bits,
            bytes,
            ..self
        }
    }

    /// Field is named if it has an ident
    fn is_named(&self) -> bool {
        self.ident.is_some()
    }

    /// Get ident of the field
    /// `index` is provided in the case of un-named structs
    /// `prefix` is true in the case of variable declarations, false if original field is desired
    fn get_ident(&self, index: usize, prefix: bool) -> TokenStream {
        let field_ident = gen_field_ident(self.ident.as_ref(), index, prefix);
        quote! { #field_ident }
    }

    /// Get the ident of the length field provided via the `len` attribute
    /// `index` is provided in the case of un-named structs
    /// `prefix` is true in the case of variable declarations, false if original field is desired
    fn get_len_field(&self, index: usize, prefix: bool) -> Option<TokenStream> {
        self.len.as_ref().map(|field_len| {
            if self.is_named() {
                gen_field_ident(Some(field_len), index, prefix)
            } else {
                let index = field_len.parse::<usize>().unwrap_or_else(|_| {
                    panic!("could not parse `len` attribute as unnamed: {}", field_len)
                });
                gen_field_ident(None::<String>, index, prefix)
            }
        })
    }
}

/// Receiver for the variant-level attributes inside a enum
#[derive(Debug, FromVariant)]
#[darling(attributes(deku), map = "DekuVariantReceiver::init")]
struct DekuVariantReceiver {
    ident: syn::Ident,
    fields: ast::Fields<DekuFieldReceiver>,

    /// custom variant reader code
    #[darling(default, map = "option_as_tokenstream")]
    reader: Option<TokenStream>,

    /// custom variant reader code
    #[darling(default, map = "option_as_tokenstream")]
    writer: Option<TokenStream>,

    /// variant `id` value
    id: String,
}

impl DekuVariantReceiver {
    fn init(self) -> Self {
        self
    }
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
        // Valid struct
        case::struct_empty(r#"struct Test {}"#),
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

        // Invalid Struct
        #[should_panic(expected = "UnknownField(ErrorUnknownField { name: \"sbits\", did_you_mean: Some(\"bits\") })")]
        case::invalid_field(r#"struct Test(#[deku(sbits=4)] u8);"#),
        #[should_panic(expected = "DuplicateField(\"bits\")")]
        case::invalid_field_duplicate(r#"struct Test(#[deku(bits=4, bits=5)] u8);"#),
        #[should_panic(expected = "conflicting: both \"bits\" and \"bytes\" specified on field")]
        case::invalid_field_bitsnbytes(r#"struct Test(#[deku(bits=4, bytes=1)] u8);"#),
        #[should_panic(expected = "`id_*` attributes only supported on enum")]
        case::invalid_struct_id_type(r#"#[deku(id_type="u8")] struct Test(u8);"#),
        #[should_panic(expected = "could not parse `len` attribute as unnamed: asd")]
        case::invalid_len_field(r#"struct Test(u8, #[deku(len="asd")] Vec<u8>);"#),

        // Valid Enum
        case::enum_empty(r#"#[deku(id_type = "u8")] enum Test {}"#),
        case::enum_all(r#"
        #[deku(id_type = "u8")]
        enum Test {
            #[deku(id = "1")]
            A,
            #[deku(id = "2")]
            B(#[deku(bits = 4)] u8),
            #[deku(id = "3")]
            C { field_n: u8 },
        }"#),

        // Invalid Enum
        #[should_panic(expected = "expected `id_type` on enum")]
        case::invalid_expected_id_type(r#"enum Test { #[deku(id="1")] A }"#),
        #[should_panic(expected = "`id_type` must be specified with `id_bits` or `id_bytes`")]
        case::invalid_expected_id_type(r#"#[deku(id_bits="5")] enum Test { #[deku(id="1")] A }"#),
        #[should_panic(expected = "`id_type` must be specified with `id_bits` or `id_bytes`")]
        case::invalid_expected_id_type(r#"#[deku(id_bytes="5")] enum Test { #[deku(id="1")] A }"#),
        #[should_panic(expected = "conflicting: both \"id_bits\" and \"id_bytes\" specified on field")]
        case::invalid_conflict(r#"#[deku(id_type="u8", id_bytes="5", id_bits="5")] enum Test { #[deku(id="1")] A }"#),
        #[should_panic(expected = "MissingField(\"id\")")]
        case::invalid_expected_id(r#"#[deku(id_type="u8")] enum Test { A }"#),

        // TODO: these tests should error/warn eventually?
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
