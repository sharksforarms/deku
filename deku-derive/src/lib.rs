use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
mod macros;
use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

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

/// # Note
/// We use this instead of `DekuReceiver::init` because handle everything in one struct is hard to use,
/// and can't save a different type i.e. `ctx: syn::LitStr` -> `ctx: syn::punctuated::Punctuated<FnArg, syn::token::Comma>`.
#[derive(Debug)]
struct DekuData {
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<VariantData, FieldData>,

    endian: EndianNess,

    ctx: Option<syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>>,

    id_type: Option<syn::Ident>,

    id_bits: Option<usize>,
}

impl DekuData {
    /// Map `DekuReceiver` to `DekuData`. It will check if attributes valid. Return a compile error
    /// if failed.
    fn from_receiver(receiver: DekuReceiver) -> Result<Self, TokenStream> {
        // Validate
        DekuData::validate(&receiver)
            .map_err(|(span, msg)| syn::Error::new(span, msg).to_compile_error())?;

        let data = match receiver.data {
            ast::Data::Struct(fields) => ast::Data::Struct(ast::Fields {
                style: fields.style,
                fields: fields
                    .fields
                    .into_iter()
                    .map(FieldData::from_receiver)
                    .collect::<Result<Vec<_>, _>>()?,
            }),
            ast::Data::Enum(variants) => ast::Data::Enum(
                variants
                    .into_iter()
                    .map(VariantData::from_receiver)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        };

        let ctx = receiver
            .ctx
            .map(|s| s.parse_with(syn::punctuated::Punctuated::parse_terminated))
            .transpose()
            .map_err(|e| e.to_compile_error())?;

        let id_bits = receiver.id_bytes.map(|b| b * 8).or(receiver.id_bits);

        Ok(Self {
            vis: receiver.vis,
            ident: receiver.ident,
            generics: receiver.generics,
            data,
            endian: receiver.endian,
            ctx,
            id_type: receiver.id_type,
            id_bits,
        })
    }

    fn validate(receiver: &DekuReceiver) -> Result<(), (proc_macro2::Span, &str)> {
        match receiver.data {
            ast::Data::Struct(_) => {
                // Validate id_* attributes are being used on an enum
                if receiver.id_type.is_some() {
                    Err((receiver.id_type.span(), "`id_type` only supported on enum"))
                } else if receiver.id_bytes.is_some() {
                    Err((
                        receiver.id_bytes.span(),
                        "`id_bytes` only supported on enum",
                    ))
                } else if receiver.id_bits.is_some() {
                    Err((receiver.id_bits.span(), "`id_bits` only supported on enum"))
                } else {
                    Ok(())
                }
            }
            ast::Data::Enum(_) => {
                // Validate either `id_bits` or `id_bytes` is specified
                if (receiver.id_bits.is_some() || receiver.id_bytes.is_some())
                    && receiver.id_type.is_none()
                {
                    return Err((
                        receiver.ident.span(),
                        "`id_type` must be specified with `id_bits` or `id_bytes`",
                    ));
                }

                // Validate either `id_bits` or `id_bytes` is specified
                if receiver.id_bits.is_some() && receiver.id_bytes.is_some() {
                    return Err((
                        receiver.id_bits.span(),
                        "conflicting: both \"id_bits\" and \"id_bytes\" specified on field",
                    ));
                }

                Ok(())
            }
        }
    }

    /// Emit a reader. If any error happened, the result will be a compile error.
    fn emit_reader(&self) -> TokenStream {
        match self.emit_reader_checked() {
            Ok(tks) => tks,
            Err(e) => e.to_compile_error(),
        }
    }

    /// Emit a writer. If any error happened, the result will be a compile error.
    fn emit_writer(&self) -> TokenStream {
        match self.emit_writer_checked() {
            Ok(tks) => tks,
            Err(e) => e.to_compile_error(),
        }
    }

    /// Same as `emit_reader`, but won't auto convert error to compile error.
    fn emit_reader_checked(&self) -> Result<TokenStream, syn::Error> {
        emit_deku_read(self)
    }

    /// Same as `emit_writer`, but won't auto convert error to compile error.
    fn emit_writer_checked(&self) -> Result<TokenStream, syn::Error> {
        emit_deku_write(self)
    }
}

#[derive(Debug)]
struct FieldData {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    /// Endianness for the field
    endian: Option<EndianNess>,

    /// field bit size
    bits: Option<usize>,

    /// tokens providing the length of the container
    count: Option<TokenStream>,

    /// apply a function to the field after it's read
    map: Option<TokenStream>,

    /// context passed to the type
    ctx: Option<Punctuated<syn::Expr, syn::token::Comma>>,

    /// map field when updating struct
    update: Option<TokenStream>,

    /// custom field reader code
    reader: Option<TokenStream>,

    /// custom field writer code
    writer: Option<TokenStream>,

    // skip field reading/writing
    skip: bool,

    // default value code when used with skip
    default: Option<TokenStream>,
}

impl FieldData {
    fn from_receiver(receiver: DekuFieldReceiver) -> Result<Self, TokenStream> {
        FieldData::validate(&receiver)
            .map_err(|(span, msg)| syn::Error::new(span, msg).to_compile_error())?;

        let bits = receiver.bytes.map(|b| b * 8).or(receiver.bits);

        // Default `default` if skip is provided without `default`
        let default = if receiver.skip && receiver.default.is_none() {
            Some(quote! { Default::default() })
        } else {
            receiver.default
        };

        let ctx = receiver
            .ctx
            .map(|s| s.parse_with(Punctuated::parse_terminated))
            .transpose()
            .map_err(|e| e.to_compile_error())?;

        Ok(Self {
            ident: receiver.ident,
            ty: receiver.ty,
            endian: receiver.endian,
            bits,
            count: receiver.count,
            map: receiver.map,
            ctx,
            update: receiver.update,
            reader: receiver.reader,
            writer: receiver.writer,
            skip: receiver.skip,
            default,
        })
    }

    fn validate(receiver: &DekuFieldReceiver) -> Result<(), (proc_macro2::Span, &str)> {
        // Validate either `bits` or `bytes` is specified
        if receiver.bits.is_some() && receiver.bytes.is_some() {
            // FIXME: Ideally we need to use `Span::join` to encompass `bits` and `bytes` together.
            return Err((
                receiver.bits.span(),
                "conflicting: both \"bits\" and \"bytes\" specified on field",
            ));
        }

        // Validate `skip` is provided with `default`
        if receiver.default.is_some() && !receiver.skip {
            return Err((
                receiver.default.span(),
                "`default` attribute must be used with `skip`",
            ));
        }

        Ok(())
    }

    /// Get ident of the field
    /// `index` is provided in the case of un-named structs
    /// `prefix` is true in the case of variable declarations, false if original field is desired
    fn get_ident(&self, index: usize, prefix: bool) -> TokenStream {
        let field_ident = gen_field_ident(self.ident.as_ref(), index, prefix);
        quote! { #field_ident }
    }
}

#[derive(Debug)]
struct VariantData {
    ident: syn::Ident,
    fields: ast::Fields<FieldData>,

    /// custom variant reader code
    reader: Option<TokenStream>,

    /// custom variant reader code
    writer: Option<TokenStream>,

    /// variant `id` value
    id: Option<String>,
}

impl VariantData {
    fn from_receiver(receiver: DekuVariantReceiver) -> Result<Self, TokenStream> {
        let fields = ast::Fields {
            style: receiver.fields.style,
            fields: receiver
                .fields
                .fields
                .into_iter()
                .map(FieldData::from_receiver)
                .collect::<Result<Vec<_>, _>>()?,
        };

        Ok(Self {
            ident: receiver.ident,
            fields,
            reader: receiver.reader,
            writer: receiver.writer,
            id: receiver.id,
        })
    }
}

/// Receiver for the top-level struct or enum
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(deku), supports(struct_any, enum_any))]
struct DekuReceiver {
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<DekuVariantReceiver, DekuFieldReceiver>,

    /// Endian default for the fields
    #[darling(default)]
    endian: EndianNess,

    /// struct/enum level ctx like "a: u8, b: u8"
    /// The type of it should be `syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>`. `darling`
    /// can't parse it from `Meta`, so we will parse it latter.
    #[darling(default)]
    ctx: Option<syn::LitStr>,

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
#[darling(attributes(deku))]
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

    /// tokens providing the length of the container
    #[darling(default, map = "option_as_tokenstream")]
    count: Option<TokenStream>,

    /// apply a function to the field after it's read
    #[darling(default, map = "option_as_tokenstream")]
    map: Option<TokenStream>,

    /// context like `"a, c + 1"`. We will parse it to `Punctuated<Expr, Comma>` latter.
    #[darling(default)]
    ctx: Option<syn::LitStr>,

    /// map field when updating struct
    #[darling(default, map = "option_as_tokenstream")]
    update: Option<TokenStream>,

    /// custom field reader code
    #[darling(default, map = "option_as_tokenstream")]
    reader: Option<TokenStream>,

    /// custom field writer code
    #[darling(default, map = "option_as_tokenstream")]
    writer: Option<TokenStream>,

    // skip field reading/writing
    #[darling(default)]
    skip: bool,

    // default value code when used with skip
    #[darling(default, map = "option_as_tokenstream")]
    default: Option<TokenStream>,
}

/// Receiver for the variant-level attributes inside a enum
#[derive(Debug, FromVariant)]
#[darling(attributes(deku))]
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
    #[darling(default)]
    id: Option<String>,
}

#[proc_macro_derive(DekuRead, attributes(deku))]
pub fn proc_deku_read(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match syn::parse(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    let receiver = match DekuReceiver::from_derive_input(&input) {
        Ok(receiver) => receiver,
        Err(err) => return err.write_errors().into(),
    };

    let data = match DekuData::from_receiver(receiver) {
        Ok(data) => data,
        Err(err) => return err.into(),
    };

    data.emit_reader().into()
}

#[proc_macro_derive(DekuWrite, attributes(deku))]
pub fn proc_deku_write(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match syn::parse(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    let receiver = match DekuReceiver::from_derive_input(&input) {
        Ok(receiver) => receiver,
        Err(err) => return err.write_errors().into(),
    };

    let data = match DekuData::from_receiver(receiver) {
        Ok(data) => data,
        Err(err) => return err.into(),
    };

    data.emit_writer().into()
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
            #[deku(skip, default = "5")]
            field_e: u32,
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
        #[should_panic(expected = "`default` attribute must be used with `skip`")]
        case::invalid_default(r#"struct Test(u8, #[deku(default ="asd")] Vec<u8>);"#),

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

        // TODO: these tests should error/warn eventually?
        // error: trying to store 9 bits in 8 bit type
        case::invalid_storage(r#"struct Test(#[deku(bits=9)] u8);"#),
        // warn: trying to set endian on a type which wouldn't make a difference
        case::invalid_endian(r#"struct Test(#[endian=big] u8);"#),
    )]
    fn test_macro(input: &str) {
        let parsed = parse_str(input).unwrap();

        let receiver = DekuReceiver::from_derive_input(&parsed).unwrap();
        let data = DekuData::from_receiver(receiver).unwrap();
        let res_reader = data.emit_reader_checked();
        let res_writer = data.emit_writer_checked();

        res_reader.unwrap();
        res_writer.unwrap();
    }
}
