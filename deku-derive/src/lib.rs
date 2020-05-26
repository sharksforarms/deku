use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
mod macros;
use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};

#[derive(Debug, Clone, Copy, PartialEq, FromMeta)]
#[darling(default)]
pub(crate) enum EndianNess {
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
#[darling(
    attributes(deku),
    supports(struct_any, enum_any),
    map = "DekuReceiver::init"
)]
struct DekuReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<DekuVariantReceiver, DekuFieldReceiver>,

    #[darling(default)]
    endian: EndianNess,

    #[darling(default)]
    id_type: Option<syn::Ident>,

    #[darling(default)]
    id_bits: Option<usize>,

    #[darling(default)]
    id_bytes: Option<usize>,
}

impl DekuReceiver {
    fn init(self) -> Self {
        if (self.id_type.is_some() || self.id_bits.is_some() || self.id_bytes.is_some())
            && !self.data.is_enum()
        {
            panic!("`id_*` attributes only supported on enum")
        }

        // Validate that `id_type` is set with size
        if (self.id_bits.is_some() || self.id_bytes.is_some()) && self.id_type.is_none() {
            panic!("`id_type` must be specified with `id_bits` or `id_bytes`");
        }

        // Validate `id_bits` and `id_bytes` attributes
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

fn option_as_tokenstream(input: Option<String>) -> Option<TokenStream> {
    input.map(|v| {
        v.parse::<TokenStream>()
            .expect("could not parse token stream")
    })
}

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

#[derive(Debug, FromField)]
#[darling(attributes(deku), map = "DekuFieldReceiver::init")]
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

    #[darling(default, map = "option_as_tokenstream")]
    reader: Option<TokenStream>,

    #[darling(default, map = "option_as_tokenstream")]
    writer: Option<TokenStream>,
}

impl DekuFieldReceiver {
    fn init(self) -> Self {
        // Validate `bits` and `bytes` attributes
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

    fn is_named(&self) -> bool {
        self.ident.is_some()
    }

    fn get_ident(&self, i: usize, prefix: bool) -> TokenStream {
        let field_ident = gen_field_ident(self.ident.as_ref(), i, prefix);
        quote! { #field_ident }
    }

    fn get_len_field(&self, i: usize, prefix: bool) -> Option<TokenStream> {
        self.len
            .as_ref()
            .map(|field_len| {
                if self.is_named() {
                    gen_field_ident(Some(field_len), i, prefix)
                } else {
                    let index = field_len.parse::<usize>().unwrap_or_else(|_| {
                        panic!("could not parse `len` attribute as unnamed: {}", field_len)
                    });
                    gen_field_ident(None::<String>, index, prefix)
                }
            })
            .map(|field_len| {
                quote! { #field_len }
            })
    }
}

#[derive(Debug, FromVariant)]
#[darling(attributes(deku), map = "DekuVariantReceiver::init")]
struct DekuVariantReceiver {
    ident: syn::Ident,
    fields: ast::Fields<DekuFieldReceiver>,

    #[darling(default)]
    endian: Option<EndianNess>,

    #[darling(default, map = "option_as_tokenstream")]
    reader: Option<TokenStream>,

    #[darling(default, map = "option_as_tokenstream")]
    writer: Option<TokenStream>,

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

        // Valid Enum
        case::enum_unnamed(r#"
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
        #[should_panic(expected = "MissingField(\"id\")")]
        case::invalid_expected_id_type(r#"#[deku(id_type="u8")] enum Test { A }"#),

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
