use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
mod macros;
use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

/// A post-processed version of `DekuReceiver`
#[derive(Debug)]
struct DekuData {
    vis: syn::Visibility,
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<VariantData, FieldData>,

    /// Endianness for all fields
    endian: Option<syn::LitStr>,

    /// top-level context, argument list
    ctx: Option<syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>>,

    /// default context passed to the field
    ctx_default: Option<Punctuated<syn::Expr, syn::token::Comma>>,

    /// enum only: `id` value
    id: Option<TokenStream>,

    /// enum only: type of the enum `id`
    id_type: Option<syn::Ident>,

    /// enum only: bit size of the enum `id`
    /// `id_bytes` is converted to `id_bits` if provided
    id_bits: Option<usize>,
}

impl DekuData {
    /// Map a `DekuReceiver` to `DekuData`
    /// It will check if attributes are valid, returns a compiler error if not
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

        let ctx_default = receiver
            .ctx_default
            .map(|s| s.parse_with(Punctuated::parse_terminated))
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
            ctx_default,
            id: receiver.id,
            id_type: receiver.id_type,
            id_bits,
        })
    }

    fn validate(receiver: &DekuReceiver) -> Result<(), (proc_macro2::Span, &str)> {
        /*
        FIXME: Issue with `span`, see `FieldData::validate`.
        */

        // Validate `ctx_default`
        if receiver.ctx_default.is_some() && receiver.ctx.is_none() {
            return Err((
                receiver.ctx_default.span(),
                "`ctx_default` must be used with `ctx`",
            ));
        }

        match receiver.data {
            ast::Data::Struct(_) => {
                // Validate id_* attributes are being used on an enum
                if receiver.id_type.is_some() {
                    Err((receiver.id_type.span(), "`id_type` only supported on enum"))
                } else if receiver.id.is_some() {
                    Err((receiver.id.span(), "`id` only supported on enum"))
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
                // Validate `id_type` or `id` is specified
                if receiver.id_type.is_none() && receiver.id.is_none() {
                    return Err((
                        receiver.ident.span(),
                        "`id_type` or `id` must be specified on enum",
                    ));
                }

                // Validate either `id_type` or `id` is specified
                if receiver.id_type.is_some() && receiver.id.is_some() {
                    return Err((
                        receiver.ident.span(),
                        "conflicting: both `id_type` and `id` specified on enum",
                    ));
                }

                // Validate `id_*` used correctly
                if receiver.id.is_some() && receiver.id_bits.is_some() {
                    return Err((
                        receiver.ident.span(),
                        "error: cannot use `id_bits` with `id`",
                    ));
                }
                if receiver.id.is_some() && receiver.id_bytes.is_some() {
                    return Err((
                        receiver.ident.span(),
                        "error: cannot use `id_bytes` with `id`",
                    ));
                }

                // Validate either `id_bits` or `id_bytes` is specified
                if receiver.id_bits.is_some() && receiver.id_bytes.is_some() {
                    return Err((
                        receiver.id_bits.span(),
                        "conflicting: both `id_bits` and `id_bytes` specified on enum",
                    ));
                }

                Ok(())
            }
        }
    }

    /// Emit a reader. On error, a compiler error is emitted
    fn emit_reader(&self) -> TokenStream {
        match self.emit_reader_checked() {
            Ok(tks) => tks,
            Err(e) => e.to_compile_error(),
        }
    }

    /// Emit a writer. On error, a compiler error is emitted
    fn emit_writer(&self) -> TokenStream {
        match self.emit_writer_checked() {
            Ok(tks) => tks,
            Err(e) => e.to_compile_error(),
        }
    }

    /// Same as `emit_reader`, but won't auto convert error to compile error
    fn emit_reader_checked(&self) -> Result<TokenStream, syn::Error> {
        emit_deku_read(self)
    }

    /// Same as `emit_writer`, but won't auto convert error to compile error
    fn emit_writer_checked(&self) -> Result<TokenStream, syn::Error> {
        emit_deku_write(self)
    }
}

/// A post-processed version of `FieldReceiver`
#[derive(Debug)]
struct FieldData {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    /// endianness for the field
    endian: Option<syn::LitStr>,

    /// field bit size
    bits: Option<usize>,

    /// tokens providing the length of the container
    count: Option<TokenStream>,

    /// apply a function to the field after it's read
    map: Option<TokenStream>,

    /// context passed to the field
    ctx: Option<Punctuated<syn::Expr, syn::token::Comma>>,

    /// map field when updating struct
    update: Option<TokenStream>,

    /// custom field reader code
    reader: Option<TokenStream>,

    /// custom field writer code
    writer: Option<TokenStream>,

    /// skip field reading/writing
    skip: bool,

    /// default value code when used with skip or cond
    default: TokenStream,

    /// condition to parse field
    cond: Option<TokenStream>,
}

impl FieldData {
    fn from_receiver(receiver: DekuFieldReceiver) -> Result<Self, TokenStream> {
        FieldData::validate(&receiver)
            .map_err(|(span, msg)| syn::Error::new(span, msg).to_compile_error())?;

        let bits = receiver.bytes.map(|b| b * 8).or(receiver.bits);

        let default = receiver.default.unwrap_or(quote! { Default::default() });

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
            cond: receiver.cond,
        })
    }

    fn validate(receiver: &DekuFieldReceiver) -> Result<(), (proc_macro2::Span, &str)> {
        // Validate either `bits` or `bytes` is specified
        if receiver.bits.is_some() && receiver.bytes.is_some() {
            /*
            FIXME: `receiver.bits.span()` will return `call_site`, that's unexpected. The compiler
               error gives:    `#[derive(DekuRead)]`
                                         ^^^^^^^^
               instead of `#[deku(bits = "", bytes = "")]`
                                  ^^^^^^^^^^^^^^^^^^^^^
               A possible reason might be that the `span` was discarded by `darling`(because inner
               type don't have a `span`).
               Maybe we should parse it manually.
            */

            // FIXME: Ideally we need to use `Span::join` to encompass `bits` and `bytes` together.
            return Err((
                receiver.bits.span(),
                "conflicting: both `bits` and `bytes` specified on field",
            ));
        }

        // Validate usage of `default` attribute
        if receiver.default.is_some() && (!receiver.skip && receiver.cond.is_none()) {
            // FIXME: Same issue with `receiver.bits.span()` see above.
            return Err((
                receiver.default.span(),
                "`default` attribute cannot be used here",
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

/// A post-processed version of `VariantReceiver`
#[derive(Debug)]
struct VariantData {
    ident: syn::Ident,
    fields: ast::Fields<FieldData>,

    /// custom variant reader code
    reader: Option<TokenStream>,

    /// custom variant reader code
    writer: Option<TokenStream>,

    /// variant `id` value
    id: Option<TokenStream>,

    /// variant `id_pat` value
    id_pat: Option<TokenStream>,
}

impl VariantData {
    fn from_receiver(receiver: DekuVariantReceiver) -> Result<Self, TokenStream> {
        VariantData::validate(&receiver)
            .map_err(|(span, msg)| syn::Error::new(span, msg).to_compile_error())?;

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
            id_pat: receiver.id_pat,
        })
    }

    fn validate(receiver: &DekuVariantReceiver) -> Result<(), (proc_macro2::Span, &str)> {
        if receiver.id.is_some() && receiver.id_pat.is_some() {
            /*
            FIXME: Issue with `span`, see `FieldData::validate`.
            */
            return Err((
                receiver.id.span(),
                "conflicting: both `id` and `id_pat` specified on variant",
            ));
        }

        Ok(())
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

    /// Endianness for all fields
    #[darling(default)]
    endian: Option<syn::LitStr>,

    /// top-level context, argument list
    // TODO: The type of it should be
    //       `syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>`
    //       https://github.com/TedDriggs/darling/pull/98
    #[darling(default)]
    ctx: Option<syn::LitStr>,

    /// default context passed to the field
    // TODO: The type of it should be
    //       `syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>`
    //       https://github.com/TedDriggs/darling/pull/98
    #[darling(default)]
    ctx_default: Option<syn::LitStr>,

    /// enum only: `id` value
    #[darling(default, map = "option_as_tokenstream")]
    id: Option<TokenStream>,

    /// enum only: type of the enum `id`
    #[darling(default)]
    id_type: Option<syn::Ident>,

    /// enum only: bit size of the enum `id`
    #[darling(default)]
    id_bits: Option<usize>,

    /// enum only: byte size of the enum `id`
    #[darling(default)]
    id_bytes: Option<usize>,
}

/// Parse a TokenStream from an Option<LitStr>
fn option_as_tokenstream(input: Option<syn::LitStr>) -> Option<TokenStream> {
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
    endian: Option<syn::LitStr>,

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

    /// context passed to the field.
    /// A comma separated argument list.
    // TODO: The type of it should be `Punctuated<Expr, Comma>`
    //       https://github.com/TedDriggs/darling/pull/98
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

    /// skip field reading/writing
    #[darling(default)]
    skip: bool,

    /// default value code when used with skip
    #[darling(default, map = "option_as_tokenstream")]
    default: Option<TokenStream>,

    /// condition to parse field
    #[darling(default, map = "option_as_tokenstream")]
    cond: Option<TokenStream>,
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
    #[darling(default, map = "option_as_tokenstream")]
    id: Option<TokenStream>,

    /// variant `id_pat` value
    #[darling(default, map = "option_as_tokenstream")]
    id_pat: Option<TokenStream>,
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
