/*!
    Procedural macros that implement `DekuRead` and `DekuWrite` traits
*/

#![warn(missing_docs)]

use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};
use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned, AttributeArgs};

mod macros;

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

    /// A magic value that must appear at the start of this struct/enum's data
    magic: Option<syn::LitByteStr>,

    /// enum only: `id` value
    id: Option<TokenStream>,

    /// enum only: type of the enum `id`
    id_type: Option<syn::Ident>,

    /// enum only: bit size of the enum `id`
    bits: Option<usize>,

    /// enum only: byte size of the enum `id`
    bytes: Option<usize>,
}

impl DekuData {
    /// Map a `DekuReceiver` to `DekuData`
    /// It will check if attributes are valid, returns a compiler error if not
    fn from_receiver(receiver: DekuReceiver) -> Result<Self, TokenStream> {
        // Validate
        DekuData::validate(&receiver)
            .map_err(|(span, msg)| syn::Error::new(span, msg).to_compile_error())?;

        let data = match receiver.data {
            ast::Data::Struct(fields) => ast::Data::Struct(ast::Fields::new(
                fields.style,
                fields
                    .fields
                    .into_iter()
                    .map(FieldData::from_receiver)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
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

        Ok(Self {
            vis: receiver.vis,
            ident: receiver.ident,
            generics: receiver.generics,
            data,
            endian: receiver.endian,
            ctx,
            ctx_default,
            magic: receiver.magic,
            id: receiver.id,
            id_type: receiver.id_type,
            bits: receiver.bits,
            bytes: receiver.bytes,
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
                    Err((receiver.id_type.span(), "`type` only supported on enum"))
                } else if receiver.id.is_some() {
                    Err((receiver.id.span(), "`id` only supported on enum"))
                } else if receiver.bytes.is_some() {
                    Err((receiver.bytes.span(), "`bytes` only supported on enum"))
                } else if receiver.bits.is_some() {
                    Err((receiver.bits.span(), "`bits` only supported on enum"))
                } else {
                    Ok(())
                }
            }
            ast::Data::Enum(_) => {
                // Validate `type` or `id` is specified
                if receiver.id_type.is_none() && receiver.id.is_none() {
                    return Err((
                        receiver.ident.span(),
                        "`type` or `id` must be specified on enum",
                    ));
                }

                // Validate either `type` or `id` is specified
                if receiver.id_type.is_some() && receiver.id.is_some() {
                    return Err((
                        receiver.ident.span(),
                        "conflicting: both `type` and `id` specified on enum",
                    ));
                }

                // Validate `id_*` used correctly
                if receiver.id.is_some() && receiver.bits.is_some() {
                    return Err((receiver.ident.span(), "error: cannot use `bits` with `id`"));
                }
                if receiver.id.is_some() && receiver.bytes.is_some() {
                    return Err((receiver.ident.span(), "error: cannot use `bytes` with `id`"));
                }

                // Validate either `bits` or `bytes` is specified
                if receiver.bits.is_some() && receiver.bytes.is_some() {
                    return Err((
                        receiver.bits.span(),
                        "conflicting: both `bits` and `bytes` specified on enum",
                    ));
                }

                Ok(())
            }
        }
    }

    /// Emit a reader. On error, a compiler error is emitted
    fn emit_reader(&self) -> TokenStream {
        self.emit_reader_checked()
            .map_or_else(|e| e.to_compile_error(), |tks| tks)
    }

    /// Emit a writer. On error, a compiler error is emitted
    fn emit_writer(&self) -> TokenStream {
        self.emit_writer_checked()
            .map_or_else(|e| e.to_compile_error(), |tks| tks)
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

    /// field byte size
    bytes: Option<usize>,

    /// tokens providing the length of the container
    count: Option<TokenStream>,

    /// tokens providing the number of bits for the length of the container
    bits_read: Option<TokenStream>,

    /// tokens providing the number of bytes for the length of the container
    bytes_read: Option<TokenStream>,

    /// a predicate to decide when to stop reading elements into the container
    until: Option<TokenStream>,

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

    /// read field as temporary value, isn't stored
    temp: bool,

    /// default value code when used with skip or cond
    default: TokenStream,

    /// condition to parse field
    cond: Option<TokenStream>,
}

impl FieldData {
    fn from_receiver(receiver: DekuFieldReceiver) -> Result<Self, TokenStream> {
        FieldData::validate(&receiver)
            .map_err(|(span, msg)| syn::Error::new(span, msg).to_compile_error())?;

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
            bits: receiver.bits,
            bytes: receiver.bytes,
            count: receiver.count,
            bits_read: receiver.bits_read,
            bytes_read: receiver.bytes_read,
            until: receiver.until,
            map: receiver.map,
            ctx,
            update: receiver.update,
            reader: receiver.reader,
            writer: receiver.writer,
            skip: receiver.skip,
            temp: receiver.temp,
            default,
            cond: receiver.cond,
        })
    }

    fn validate(receiver: &DekuFieldReceiver) -> Result<(), (proc_macro2::Span, &str)> {
        // Validate either `read_bytes` or `read_bits` is specified
        if receiver.bits_read.is_some() && receiver.bytes_read.is_some() {
            return Err((
                receiver.bits_read.span(),
                "conflicting: both `bits_read` and `bytes_read` specified on field",
            ));
        }

        // Validate either `count` or `bits_read`/`bytes_read` is specified
        if receiver.count.is_some()
            && (receiver.bits_read.is_some() || receiver.bytes_read.is_some())
        {
            if receiver.bits_read.is_some() {
                return Err((
                    receiver.count.span(),
                    "conflicting: both `count` and `bits_read` specified on field",
                ));
            } else {
                return Err((
                    receiver.count.span(),
                    "conflicting: both `count` and `bytes_read` specified on field",
                ));
            }
        }

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

        let fields = ast::Fields::new(
            receiver.fields.style,
            receiver
                .fields
                .fields
                .into_iter()
                .map(FieldData::from_receiver)
                .collect::<Result<Vec<_>, _>>()?,
        );

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

        if let Some(id) = &receiver.id {
            if id.to_string() == "_" {
                return Err((
                    receiver.ident.span(),
                    "error: `id_pat` should be used for `_`",
                ));
            }
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
    #[darling(default, map = "apply_replacements")]
    ctx: Option<syn::LitStr>,

    /// default context passed to the field
    // TODO: The type of it should be
    //       `syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>`
    //       https://github.com/TedDriggs/darling/pull/98
    #[darling(default)]
    ctx_default: Option<syn::LitStr>,

    /// A magic value that must appear at the start of this struct/enum's data
    #[darling(default)]
    magic: Option<syn::LitByteStr>,

    /// enum only: `id` value
    #[darling(default, map = "option_as_tokenstream")]
    id: Option<TokenStream>,

    /// enum only: type of the enum `id`
    #[darling(rename = "type", default)]
    id_type: Option<syn::Ident>,

    /// enum only: bit size of the enum `id`
    #[darling(default)]
    bits: Option<usize>,

    /// enum only: byte size of the enum `id`
    #[darling(default)]
    bytes: Option<usize>,
}

fn apply_replacements(input: Option<syn::LitStr>) -> Option<syn::LitStr> {
    input.map(|v| {
        if v.value().contains("__deku_") {
            panic!(
                "error: attribute cannot contain `__deku_` these are internal variables. Please use the `deku::` instead."
            );
        }

        let v_str = v
            .value()
            .replace("deku::input", "__deku_input") // part of the public API `from_bytes`
            .replace("deku::input_bits", "__deku_input_bits") // part of the public API `read`
            .replace("deku::output", "__deku_output") // part of the public API `write`
            .replace("deku::rest", "__deku_rest")
            .replace("deku::bit_offset", "__deku_bit_offset")
            .replace("deku::byte_offset", "__deku_byte_offset");

        syn::LitStr::new(&v_str, v.span())
    })
}

/// Parse a TokenStream from an Option<LitStr>
/// Also replaces any namespaced variables to internal variables found in `input`
fn option_as_tokenstream(input: Option<syn::LitStr>) -> Option<TokenStream> {
    input.map(|v| {
        let v = apply_replacements(Some(v)).unwrap();
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

    /// tokens providing the number of bits for the length of the container
    #[darling(default, map = "option_as_tokenstream")]
    bits_read: Option<TokenStream>,

    /// tokens providing the number of bytes for the length of the container
    #[darling(default, map = "option_as_tokenstream")]
    bytes_read: Option<TokenStream>,

    /// a predicate to decide when to stop reading elements into the container
    #[darling(default, map = "option_as_tokenstream")]
    until: Option<TokenStream>,

    /// apply a function to the field after it's read
    #[darling(default, map = "option_as_tokenstream")]
    map: Option<TokenStream>,

    /// context passed to the field.
    /// A comma separated argument list.
    // TODO: The type of it should be `Punctuated<Expr, Comma>`
    //       https://github.com/TedDriggs/darling/pull/98
    #[darling(default, map = "apply_replacements")]
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

    /// read field as temporary value, isn't stored
    #[darling(default)]
    temp: bool,

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

/// Entry function for `DekuRead` proc-macro
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

/// Entry function for `DekuWrite` proc-macro
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

fn is_not_deku(attr: &syn::Attribute) -> bool {
    attr.path
        .get_ident()
        .map(|ident| ident != "deku" && ident != "deku_derive")
        .unwrap_or(true)
}

fn is_temp(field: &syn::Field) -> bool {
    DekuFieldReceiver::from_field(field)
        .map(|attrs| attrs.temp)
        .unwrap_or(false)
}

fn remove_deku_temp_fields(fields: &mut syn::punctuated::Punctuated<syn::Field, syn::Token![,]>) {
    *fields = fields
        .clone()
        .into_pairs()
        .filter(|x| !is_temp(x.value()))
        .collect()
}
fn remove_deku_field_attrs(fields: &mut syn::punctuated::Punctuated<syn::Field, syn::Token![,]>) {
    *fields = fields
        .clone()
        .into_pairs()
        .map(|mut field| {
            field.value_mut().attrs.retain(is_not_deku);
            field
        })
        .collect()
}

fn remove_deku_attrs(fields: &mut syn::Fields) {
    match fields {
        syn::Fields::Named(ref mut fields) => remove_deku_field_attrs(&mut fields.named),
        syn::Fields::Unnamed(ref mut fields) => remove_deku_field_attrs(&mut fields.unnamed),
        syn::Fields::Unit => (),
    }
}

fn remove_temp_fields(fields: &mut syn::Fields) {
    match fields {
        syn::Fields::Named(ref mut fields) => remove_deku_temp_fields(&mut fields.named),
        syn::Fields::Unnamed(ref mut fields) => remove_deku_temp_fields(&mut fields.unnamed),
        syn::Fields::Unit => (),
    }
}

#[derive(Debug, FromMeta)]
struct DekuDerive {
    #[darling(default, rename = "DekuRead")]
    read: bool,
    #[darling(default, rename = "DekuWrite")]
    write: bool,
}

/// Entry function for `deku_derive` proc-macro
/// This attribute macro is used to derive `DekuRead` and `DekuWrite`
/// while removing temporary variables.
#[proc_macro_attribute]
pub fn deku_derive(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut derives = vec![];

    // Parse `deku_derive` attribute
    let attr_args = syn::parse_macro_input!(attr as AttributeArgs);
    let args = match DekuDerive::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return proc_macro::TokenStream::from(e.write_errors());
        }
    };

    // Generate `DekuRead` impl
    if args.read {
        let derive: TokenStream = proc_deku_read(item.clone()).into();
        derives.push(derive);
    }

    // Remove the temp fields
    let mut input = syn::parse_macro_input!(item as syn::DeriveInput);

    match input.data {
        syn::Data::Struct(ref mut input_struct) => remove_temp_fields(&mut input_struct.fields),
        syn::Data::Enum(ref mut input_enum) => {
            for variant in input_enum.variants.iter_mut() {
                remove_temp_fields(&mut variant.fields)
            }
        }
        _ => unimplemented!(),
    }

    // Generate `DekuWrite` impl
    if args.write {
        let input = input.clone();
        let derive: TokenStream = proc_deku_write(input.into_token_stream().into()).into();
        derives.push(derive);
    }

    // Remove attributes
    match input.data {
        syn::Data::Struct(ref mut input_struct) => {
            input.attrs.retain(is_not_deku);
            remove_deku_attrs(&mut input_struct.fields)
        }
        syn::Data::Enum(ref mut input_enum) => {
            for variant in input_enum.variants.iter_mut() {
                variant.attrs.retain(is_not_deku);
                remove_deku_attrs(&mut variant.fields)
            }
        }
        _ => unimplemented!(),
    }

    input.attrs.retain(is_not_deku);

    quote!(
        #(#derives)*

        #input
    )
    .into()
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
        case::enum_empty(r#"#[deku(type = "u8")] enum Test {}"#),
        case::enum_all(r#"
        #[deku(type = "u8")]
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
