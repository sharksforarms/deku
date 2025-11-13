/*!
Procedural macros that implement `DekuRead` and `DekuWrite` traits
 */
#![warn(missing_docs)]

extern crate alloc;

use alloc::borrow::Cow;
use std::convert::TryFrom;
use std::fmt::Display;
use syn::{Attribute, Meta, Type};

use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant, ToTokens};
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::macros::deku_read::emit_deku_read;
use crate::macros::deku_write::emit_deku_write;

mod macros;

// Pad attribute values are not bounded by the implementation, however, for
// implementation purposes we need a concrete object to contain padding data.
// To allow use of padding in constrained environments we define the object as
// an array (by contrast to a vec). PAD_ARRAY_SIZE controls the length of
// these arrays.
//
// The length of the padding array is chosen with the following considerations:
//
// - A loop is necessary to avoid arbitrarily picking an upper-bound for
//   requested padding values
//
// - Given the loop there aren't any strong constraints on the size of the
//   padding array
//
// - My estimate is padding requests tend to be in the range of 1-16 bytes.
//   From a [brief inspection][github-search-pad-bytes] it seems that estimate
//   is reasonable
//
// - We desire a value large enough to cover common cases and avoid unnecessary
//   loop trips, but small enough not to cause trouble on the stack (for
//   instance, requesting a page of zeros feels unreasonable)
//
// - Try to pick something that only requires 1 execution of the loop body for
//   common cases.
//
// [github-search-pad-bytes]:
//   https://github.com/search?q=pad_bytes_before+OR+pad_bytes_after&type=code
const PAD_ARRAY_SIZE: usize = 64;

#[derive(Debug)]
enum Id {
    TokenStream(TokenStream),
    LitByteStr(syn::LitByteStr),
    Int(syn::LitInt),
    Bool(syn::LitBool),
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_token_stream().to_string())
    }
}

impl ToTokens for Id {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Id::TokenStream(v) => v.to_tokens(tokens),
            Id::LitByteStr(v) => v.to_tokens(tokens),
            Id::Int(v) => v.to_tokens(tokens),
            Id::Bool(v) => v.to_tokens(tokens),
        }
    }
}

impl FromMeta for Id {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        (match *value {
            syn::Lit::Str(ref s) => Ok(Id::TokenStream(
                apply_replacements(s)
                    .map_err(darling::Error::custom)?
                    .parse::<TokenStream>()
                    .expect("could not parse token stream"),
            )),
            syn::Lit::Int(ref s) => Ok(Id::Int(s.clone())),
            syn::Lit::Bool(ref s) => Ok(Id::Bool(s.clone())),
            syn::Lit::ByteStr(ref s) => Ok(Id::LitByteStr(s.clone())),
            _ => Err(darling::Error::unexpected_lit_type(value)),
        })
        .map_err(|e| e.with_span(value))
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(Id::TokenStream(
            value.parse().expect("Failed to parse tokens"),
        ))
    }
}

#[derive(Debug)]
enum Num {
    LitInt(syn::LitInt),
    TokenStream(TokenStream),
}

impl Num {
    fn new(n: syn::LitInt) -> Self {
        Self::LitInt(n)
    }
}

impl Display for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_token_stream().to_string())
    }
}

impl ToTokens for Num {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Num::LitInt(v) => v.to_tokens(tokens),
            Num::TokenStream(v) => v.to_tokens(tokens),
        }
    }
}

impl FromMeta for Num {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        (match *value {
            syn::Lit::Str(ref s) => {
                let str_value = s.value();

                // Attempt to read ("2") as a Num LitInt (2)
                match str_value.parse::<u64>() {
                    Ok(int_value) => {
                        let int_string = int_value.to_string();
                        let span = s.span();
                        let lit_int = syn::LitInt::new(&int_string, span);
                        Ok(Num::new(lit_int))
                    }
                    // else, just a tokenstream
                    Err(_) => Ok(Num::TokenStream(
                        apply_replacements(s)
                            .map_err(darling::Error::custom)?
                            .parse::<TokenStream>()
                            .expect("could not parse token stream"),
                    )),
                }
            }
            syn::Lit::Int(ref s) => Ok(Num::new(s.clone())),
            _ => Err(darling::Error::unexpected_lit_type(value)),
        })
        .map_err(|e| e.with_span(value))
    }
}

fn cerror(span: proc_macro2::Span, msg: &str) -> TokenStream {
    syn::Error::new(span, msg).to_compile_error()
}

/// A post-processed version of `DekuReceiver`
#[derive(Debug)]
struct DekuData {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<VariantData, FieldData>,

    repr: Option<ReprType>,

    /// Endianness for all fields
    endian: Option<syn::LitStr>,

    /// top-level context, argument list
    ctx: Option<syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>>,

    /// default context passed to the field
    ctx_default: Option<Punctuated<syn::Expr, syn::token::Comma>>,

    /// A magic value that must appear at the start of this struct/enum's data
    magic: Option<syn::LitByteStr>,

    /// enum only: `id` value
    id: Option<Id>,

    /// enum only: type of the enum `id`
    id_type: Option<TokenStream>,

    /// enum only: endianness of the enum `id`
    id_endian: Option<syn::LitStr>,

    /// enum only: bit size of the enum `id`
    #[cfg(feature = "bits")]
    bits: Option<Num>,

    /// enum only: byte size of the enum `id`
    bytes: Option<Num>,

    /// struct only: seek from current position
    seek_rewind: bool,

    /// struct only: seek from current position
    seek_from_current: Option<TokenStream>,

    /// struct only: seek from end position
    seek_from_end: Option<TokenStream>,

    /// struct only: seek from start position
    seek_from_start: Option<TokenStream>,

    /// Bit Order for all fields
    bit_order: Option<syn::LitStr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReprType {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl From<ReprType> for TokenStream {
    fn from(value: ReprType) -> Self {
        match value {
            ReprType::U8 => quote! { u8 },
            ReprType::U16 => quote! { u16 },
            ReprType::U32 => quote! { u32 },
            ReprType::U64 => quote! { u64 },
            ReprType::U128 => quote! { u128 },
            ReprType::I8 => quote! { i8 },
            ReprType::I16 => quote! { i16 },
            ReprType::I32 => quote! { i32 },
            ReprType::I64 => quote! { i64 },
            ReprType::I128 => quote! { i128 },
        }
    }
}

fn from_token(ts: TokenStream) -> Option<ReprType> {
    let mut repr_value = None;

    for token in ts.into_iter() {
        if let TokenTree::Ident(ident) = token {
            match ident.to_string().as_str() {
                "u8" => repr_value = Some(ReprType::U8),
                "u16" => repr_value = Some(ReprType::U16),
                "u32" => repr_value = Some(ReprType::U32),
                "u64" => repr_value = Some(ReprType::U64),
                "u128" => repr_value = Some(ReprType::U128),
                "i8" => repr_value = Some(ReprType::I8),
                "i16" => repr_value = Some(ReprType::I16),
                "i32" => repr_value = Some(ReprType::I32),
                "i64" => repr_value = Some(ReprType::I64),
                "i128" => repr_value = Some(ReprType::I128),
                _ => {}
            }
        }
    }

    repr_value
}

fn repr(attrs: &[Attribute]) -> Option<ReprType> {
    for attr in attrs {
        if attr.path().is_ident("repr") {
            let meta = &attr.meta;
            if let Meta::List(meta_list) = meta {
                let tokens = &meta_list.tokens;
                let token_str = tokens.to_string();
                match token_str.trim() {
                    "u8" => return Some(ReprType::U8),
                    "u16" => return Some(ReprType::U16),
                    "u32" => return Some(ReprType::U32),
                    "u64" => return Some(ReprType::U64),
                    "u128" => return Some(ReprType::U128),
                    "i8" => return Some(ReprType::I8),
                    "i16" => return Some(ReprType::I16),
                    "i32" => return Some(ReprType::I32),
                    "i64" => return Some(ReprType::I64),
                    "i128" => return Some(ReprType::I128),
                    _ => (),
                }
            }
        }
    }

    None
}

impl DekuData {
    fn from_input(input: TokenStream) -> Result<Self, TokenStream> {
        let input = match syn::parse2(input) {
            Ok(input) => input,
            Err(err) => return Err(err.to_compile_error()),
        };

        let receiver = match DekuReceiver::from_derive_input(&input) {
            Ok(receiver) => receiver,
            Err(err) => return Err(err.write_errors()),
        };

        let attrs = input.attrs;

        DekuData::from_receiver(receiver, attrs)
    }

    /// Map a `DekuReceiver` to `DekuData`
    fn from_receiver(receiver: DekuReceiver, attrs: Vec<Attribute>) -> Result<Self, TokenStream> {
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

        let repr = repr(&attrs);

        let data = Self {
            ident: receiver.ident,
            generics: receiver.generics,
            data,
            repr,
            endian: receiver.endian,
            ctx: receiver.ctx,
            ctx_default: receiver.ctx_default,
            magic: receiver.magic,
            id: receiver.id,
            id_type: receiver.id_type?,
            id_endian: receiver.id_endian,
            #[cfg(feature = "bits")]
            bits: receiver.bits,
            bytes: receiver.bytes,
            seek_rewind: receiver.seek_rewind,
            seek_from_current: receiver.seek_from_current?,
            seek_from_end: receiver.seek_from_end?,
            seek_from_start: receiver.seek_from_start?,
            bit_order: receiver.bit_order,
        };

        DekuData::validate(&data)?;

        Ok(data)
    }

    fn validate(data: &DekuData) -> Result<(), TokenStream> {
        // Validate `ctx_default`
        if data.ctx_default.is_some() && data.ctx.is_none() {
            // FIXME: Use `Span::join` once out of nightly
            return Err(cerror(
                data.ctx_default.span(),
                "`ctx_default` must be used with `ctx`",
            ));
        }

        match data.data {
            ast::Data::Struct(_) => {
                // Validate id_* attributes are being used on an enum
                let ret = if data.id_type.is_some() {
                    Err(cerror(
                        data.id_type.span(),
                        "`id_type` only supported on enum",
                    ))
                } else if data.id.is_some() {
                    Err(cerror(data.id.span(), "`id` only supported on enum"))
                } else if data.id_endian.is_some() {
                    Err(cerror(data.id.span(), "`id_endian` only supported on enum"))
                } else if data.bytes.is_some() {
                    Err(cerror(data.bytes.span(), "`bytes` only supported on enum"))
                } else {
                    Ok(())
                };

                #[cfg(feature = "bits")]
                if ret.is_ok() && data.bits.is_some() {
                    return Err(cerror(data.bits.span(), "`bits` only supported on enum"));
                }

                ret
            }
            ast::Data::Enum(_) => {
                // Validate `id_type` or `id` is specified
                if data.id_type.is_none() && data.id.is_none() {
                    return Err(cerror(
                        data.ident.span(),
                        "`id_type` or `id` must be specified on enum",
                    ));
                }

                // Validate either `id_type` or `id` is specified
                if data.id_type.is_some() && data.id.is_some() {
                    return Err(cerror(
                        data.ident.span(),
                        "conflicting: both `id_type` and `id` specified on enum",
                    ));
                }

                // Validate `id_*` used correctly
                #[cfg(feature = "bits")]
                if data.id.is_some() && data.bits.is_some() {
                    return Err(cerror(
                        data.ident.span(),
                        "error: cannot use `bits` with `id`",
                    ));
                }
                if data.id.is_some() && data.bytes.is_some() {
                    return Err(cerror(
                        data.ident.span(),
                        "error: cannot use `bytes` with `id`",
                    ));
                }

                // Validate either `bits` or `bytes` is specified
                #[cfg(feature = "bits")]
                if data.bits.is_some() && data.bytes.is_some() {
                    return Err(cerror(
                        data.bits.span(),
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
            .unwrap_or_else(|e| e.to_compile_error())
    }

    /// Emit a writer. On error, a compiler error is emitted
    fn emit_writer(&self) -> TokenStream {
        self.emit_writer_checked()
            .unwrap_or_else(|e| e.to_compile_error())
    }

    /// Same as `emit_reader`, but won't auto convert error to compile error
    fn emit_reader_checked(&self) -> Result<TokenStream, syn::Error> {
        emit_deku_read(self)
    }

    /// Same as `emit_writer`, but won't auto convert error to compile error
    fn emit_writer_checked(&self) -> Result<TokenStream, syn::Error> {
        emit_deku_write(self)
    }

    /// Emit a size implementation
    fn emit_size(&self) -> TokenStream {
        self.emit_size_checked()
            .unwrap_or_else(|e| e.to_compile_error())
    }

    /// Emit a size implementation, no compile_error
    fn emit_size_checked(&self) -> Result<TokenStream, syn::Error> {
        macros::deku_size::emit_deku_size(self)
    }
}

/// Common variables from `DekuData` for `emit_enum` read/write functions
#[derive(Debug)]
struct DekuDataEnum<'a> {
    imp: syn::ImplGenerics<'a>,
    wher: Option<&'a syn::WhereClause>,
    variants: Vec<&'a VariantData>,
    ident: TokenStream,
    id: Option<&'a Id>,
    id_type: Option<&'a TokenStream>,
    id_args: TokenStream,
}

impl<'a> TryFrom<&'a DekuData> for DekuDataEnum<'a> {
    type Error = syn::Error;

    /// Create common initializer variables for `emit_enum` read/write functions
    fn try_from(deku_data: &'a DekuData) -> Result<Self, Self::Error> {
        let (imp, ty, wher) = deku_data.generics.split_for_impl();

        // checked in `emit_deku_{read/write}`
        let variants = deku_data.data.as_ref().take_enum().unwrap();

        let ident = &deku_data.ident;
        let ident = quote! { #ident #ty };

        let id = deku_data.id.as_ref();
        let id_type = deku_data.id_type.as_ref();

        let id_args = crate::macros::gen_id_args(
            deku_data.endian.as_ref(),
            deku_data.id_endian.as_ref(),
            #[cfg(feature = "bits")]
            deku_data.bits.as_ref(),
            #[cfg(not(feature = "bits"))]
            None,
            deku_data.bytes.as_ref(),
            deku_data.bit_order.as_ref(),
        )?;

        Ok(Self {
            imp,
            wher,
            variants,
            ident,
            id,
            id_type,
            id_args,
        })
    }
}

/// Common variables from `DekuData` for `emit_struct` read/write functions
#[derive(Debug)]
struct DekuDataStruct<'a> {
    imp: syn::ImplGenerics<'a>,
    wher: Option<&'a syn::WhereClause>,
    ident: TokenStream,
    fields: darling::ast::Fields<&'a FieldData>,
}

impl<'a> TryFrom<&'a DekuData> for DekuDataStruct<'a> {
    type Error = syn::Error;

    /// Create common initializer variables for `emit_struct` read/write functions
    fn try_from(deku_data: &'a DekuData) -> Result<Self, Self::Error> {
        let (imp, ty, wher) = deku_data.generics.split_for_impl();

        let ident = &deku_data.ident;
        let ident = quote! { #ident #ty };

        // Checked in `emit_deku_{read/write}`.
        let fields = deku_data.data.as_ref().take_struct().unwrap();

        Ok(Self {
            imp,
            wher,
            ident,
            fields,
        })
    }
}

/// A post-processed version of `FieldReceiver`
#[derive(Debug)]
struct FieldData {
    ident: Option<syn::Ident>,
    ty: Type,

    /// endianness for the field
    endian: Option<syn::LitStr>,

    /// field bit size
    #[cfg(feature = "bits")]
    bits: Option<Num>,

    /// field byte size
    bytes: Option<Num>,

    /// tokens providing the length of the container
    count: Option<TokenStream>,

    /// tokens providing the number of bits for the length of the container
    #[cfg(feature = "bits")]
    bits_read: Option<TokenStream>,

    /// tokens providing the number of bytes for the length of the container
    bytes_read: Option<TokenStream>,

    /// a predicate to decide when to stop reading elements into the container
    until: Option<TokenStream>,

    /// read until `reader.end()`
    read_all: bool,

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

    /// pad a number of bits before
    #[cfg(feature = "bits")]
    pad_bits_before: Option<TokenStream>,

    /// pad a number of bytes before
    pad_bytes_before: Option<TokenStream>,

    /// pad a number of bits after
    #[cfg(feature = "bits")]
    pad_bits_after: Option<TokenStream>,

    /// pad a number of bytes after
    pad_bytes_after: Option<TokenStream>,

    /// read field as temporary value, isn't stored
    temp: bool,

    /// write given value of temp field
    temp_value: Option<TokenStream>,

    /// default value code when used with skip or cond
    default: Option<TokenStream>,

    /// condition to parse field
    cond: Option<TokenStream>,

    /// assertion on field
    assert: Option<TokenStream>,

    /// assert value of field
    assert_eq: Option<TokenStream>,

    /// seek from current position
    seek_rewind: bool,

    /// seek from current position
    seek_from_current: Option<TokenStream>,

    /// seek from end position
    seek_from_end: Option<TokenStream>,

    /// seek from start position
    seek_from_start: Option<TokenStream>,

    /// Bit Order of field
    bit_order: Option<syn::LitStr>,

    /// magic value that needs to appear before field
    magic: Option<syn::LitByteStr>,
}

impl FieldData {
    pub fn any_field_set(&self) -> bool {
        // NOTE: Ignore ident
        let mut any_option_set = self.endian.is_some();

        #[cfg(feature = "bits")]
        {
            any_option_set = any_option_set || self.bits.is_some();
        }

        any_option_set = any_option_set || self.bytes.is_some() || self.count.is_some();

        #[cfg(feature = "bits")]
        {
            any_option_set = any_option_set || self.bits_read.is_some();
        }

        any_option_set = any_option_set
            || self.bytes_read.is_some()
            || self.until.is_some()
            || self.map.is_some()
            || self.ctx.is_some()
            || self.update.is_some()
            || self.reader.is_some()
            || self.writer.is_some();

        #[cfg(feature = "bits")]
        {
            any_option_set = any_option_set || self.pad_bits_before.is_some();
        }

        any_option_set = any_option_set || self.pad_bytes_before.is_some();

        #[cfg(feature = "bits")]
        {
            any_option_set = any_option_set || self.pad_bits_after.is_some();
        }

        // NOTE: Ignore default
        any_option_set = any_option_set
            || self.pad_bytes_after.is_some()
            || self.temp_value.is_some()
            || self.cond.is_some()
            || self.assert.is_some()
            || self.assert_eq.is_some()
            || self.seek_from_current.is_some()
            || self.seek_from_end.is_some()
            || self.seek_from_start.is_some()
            || self.bit_order.is_some()
            || self.magic.is_some();

        let any_bool_set = self.read_all || self.skip || self.temp || self.seek_rewind;

        any_option_set || any_bool_set
    }

    fn from_receiver(receiver: DekuFieldReceiver) -> Result<Self, TokenStream> {
        let ctx = receiver
            .ctx?
            .map(|s| s.parse_with(Punctuated::parse_terminated))
            .transpose()
            .map_err(|e| e.to_compile_error())?;

        let data = Self {
            ident: receiver.ident,
            ty: receiver.ty,
            endian: receiver.endian,
            #[cfg(feature = "bits")]
            bits: receiver.bits,
            bytes: receiver.bytes,
            count: receiver.count?,
            #[cfg(feature = "bits")]
            bits_read: receiver.bits_read?,
            bytes_read: receiver.bytes_read?,
            until: receiver.until?,
            read_all: receiver.read_all,
            map: receiver.map?,
            ctx,
            update: receiver.update?,
            reader: receiver.reader?,
            writer: receiver.writer?,
            skip: receiver.skip,
            #[cfg(feature = "bits")]
            pad_bits_before: receiver.pad_bits_before?,
            pad_bytes_before: receiver.pad_bytes_before?,
            #[cfg(feature = "bits")]
            pad_bits_after: receiver.pad_bits_after?,
            pad_bytes_after: receiver.pad_bytes_after?,
            temp: receiver.temp,
            temp_value: receiver.temp_value?,
            default: receiver.default?,
            cond: receiver.cond?,
            assert: receiver.assert?,
            assert_eq: receiver.assert_eq?,
            seek_rewind: receiver.seek_rewind,
            seek_from_current: receiver.seek_from_current?,
            seek_from_end: receiver.seek_from_end?,
            seek_from_start: receiver.seek_from_start?,
            bit_order: receiver.bit_order,
            magic: receiver.magic,
        };

        FieldData::validate(&data)?;

        let default = data.default.or_else(|| Some(quote! { Default::default() }));

        Ok(Self { default, ..data })
    }

    fn validate(data: &FieldData) -> Result<(), TokenStream> {
        // Validate either `read_bytes` or `read_bits` is specified
        #[cfg(feature = "bits")]
        if data.bits_read.is_some() && data.bytes_read.is_some() {
            return Err(cerror(
                data.bits_read.span(),
                "conflicting: both `bits_read` and `bytes_read` specified on field",
            ));
        }

        // Validate either `count` or `bits_read`/`bytes_read` is specified
        #[cfg(feature = "bits")]
        if data.count.is_some() && (data.bits_read.is_some() || data.bytes_read.is_some()) {
            if data.bits_read.is_some() {
                return Err(cerror(
                    data.count.span(),
                    "conflicting: both `count` and `bits_read` specified on field",
                ));
            } else {
                return Err(cerror(
                    data.count.span(),
                    "conflicting: both `count` and `bytes_read` specified on field",
                ));
            }
        }

        #[cfg(not(feature = "bits"))]
        if data.count.is_some() && data.bytes_read.is_some() {
            return Err(cerror(
                data.count.span(),
                "conflicting: both `count` and `bytes_read` specified on field",
            ));
        }

        // Validate either `bits` or `bytes` is specified
        #[cfg(feature = "bits")]
        if data.bits.is_some() && data.bytes.is_some() {
            // FIXME: Use `Span::join` once out of nightly
            return Err(cerror(
                data.bits.span(),
                "conflicting: both `bits` and `bytes` specified on field",
            ));
        }

        // Validate usage of `default` attribute
        if data.default.is_some() && (!data.skip && data.cond.is_none()) {
            // FIXME: Use `Span::join` once out of nightly
            return Err(cerror(
                data.default.span(),
                "`default` attribute cannot be used here",
            ));
        }

        // Validate usage of read_all
        #[cfg(feature = "bits")]
        if data.read_all
            && (data.until.is_some()
                || data.count.is_some()
                || (data.bits_read.is_some() || data.bytes_read.is_some()))
        {
            return Err(cerror(
                data.read_all.span(),
                "conflicting: `read_all` cannot be used with `until`, `count`, `bits_read`, or `bytes_read`",
            ));
        }

        // Validate usage of seek_*
        if (data.seek_from_current.is_some() as u8
            + data.seek_from_end.is_some() as u8
            + data.seek_from_start.is_some() as u8
            + data.seek_rewind as u8)
            > 1
        {
            return Err(cerror(
                data.ty.span(),
                "conflicting: only one `seek` attribute can be used at one time",
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
    discriminant: Option<syn::Expr>,

    /// custom variant reader code
    reader: Option<TokenStream>,

    /// custom variant reader code
    writer: Option<TokenStream>,

    /// variant `id` value
    id: Option<Id>,

    /// variant `id_pat` value
    id_pat: Option<TokenStream>,

    /// variant `default` option
    default: Option<bool>,
}

impl VariantData {
    fn from_receiver(receiver: DekuVariantReceiver) -> Result<Self, TokenStream> {
        let fields = ast::Fields::new(
            receiver.fields.style,
            receiver
                .fields
                .fields
                .into_iter()
                .map(FieldData::from_receiver)
                .collect::<Result<Vec<_>, _>>()?,
        );

        let ret = Self {
            ident: receiver.ident,
            fields,
            discriminant: receiver.discriminant,
            reader: receiver.reader?,
            writer: receiver.writer?,
            id: receiver.id,
            id_pat: receiver.id_pat?,
            default: receiver.default,
        };

        VariantData::validate(&ret)?;

        Ok(ret)
    }

    fn validate(data: &VariantData) -> Result<(), TokenStream> {
        if data.id.is_some() && data.id_pat.is_some() {
            // FIXME: Use `Span::join` once out of nightly
            return Err(cerror(
                data.id.span(),
                "conflicting: both `id` and `id_pat` specified on variant",
            ));
        }

        if let Some(id) = &data.id {
            if id.to_string() == "_" {
                return Err(cerror(
                    data.ident.span(),
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
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<DekuVariantReceiver, DekuFieldReceiver>,

    /// Endianness for all fields
    #[darling(default)]
    endian: Option<syn::LitStr>,

    /// top-level context, argument list
    #[darling(default)]
    ctx: Option<syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>>,

    /// default context passed to the field
    #[darling(default)]
    ctx_default: Option<syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>>,

    /// A magic value that must appear at the start of this struct/enum's data
    #[darling(default)]
    magic: Option<syn::LitByteStr>,

    /// enum only: `id` value
    #[darling(default)]
    id: Option<Id>,

    /// enum only: type of the enum `id`
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    id_type: Result<Option<TokenStream>, ReplacementError>,

    /// enum only: endianness of the enum `id`
    #[darling(default)]
    id_endian: Option<syn::LitStr>,

    /// enum only: bit size of the enum `id`
    #[cfg(feature = "bits")]
    #[darling(default)]
    bits: Option<Num>,

    /// enum only: byte size of the enum `id`
    #[darling(default)]
    bytes: Option<Num>,

    /// struct only: seek from current position
    #[darling(default)]
    seek_rewind: bool,

    /// struct only: seek from current position
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    seek_from_current: Result<Option<TokenStream>, ReplacementError>,

    /// struct only: seek from end position
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    seek_from_end: Result<Option<TokenStream>, ReplacementError>,

    /// struct only: seek from start position
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    seek_from_start: Result<Option<TokenStream>, ReplacementError>,

    /// Bit Order of field
    #[darling(default)]
    bit_order: Option<syn::LitStr>,
}

type ReplacementError = TokenStream;

fn apply_replacements(input: &syn::LitStr) -> Result<Cow<'_, syn::LitStr>, ReplacementError> {
    let input_value = input.value();

    if !input_value.contains("deku") {
        return Ok(Cow::Borrowed(input));
    }

    if input_value.contains("__deku_") {
        return Err(darling::Error::unsupported_format(
            "attribute cannot contain `__deku_` these are internal variables. Please use the `deku::` instead."
        )).map_err(|e| e.with_span(&input).write_errors());
    }

    let input_str = input_value
        .replace("deku::reader", "__deku_reader")
        .replace("deku::writer", "__deku_writer")
        .replace("deku::bit_offset", "__deku_bit_offset")
        .replace("deku::byte_offset", "__deku_byte_offset");

    Ok(Cow::Owned(syn::LitStr::new(&input_str, input.span())))
}

/// Calls apply replacements on Option<LitStr>
fn map_option_litstr(input: Option<syn::LitStr>) -> Result<Option<syn::LitStr>, ReplacementError> {
    Ok(match input {
        Some(v) => Some(apply_replacements(&v)?.into_owned()),
        None => None,
    })
}

/// Parse a TokenStream from an Option<LitStr>
/// Also replaces any namespaced variables to internal variables found in `input`
fn map_litstr_as_tokenstream(
    input: Option<syn::LitStr>,
) -> Result<Option<TokenStream>, ReplacementError> {
    Ok(match input {
        Some(v) => {
            let v = apply_replacements(&v)?;
            Some(
                v.parse::<TokenStream>()
                    .expect("could not parse token stream"),
            )
        }
        None => None,
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

/// Provided default when a attribute is not available
#[allow(clippy::unnecessary_wraps)]
fn default_res_opt<T, E>() -> Result<Option<T>, E> {
    Ok(None)
}

/// Receiver for the field-level attributes inside a struct/enum variant
#[derive(Debug, FromField)]
#[darling(attributes(deku))]
struct DekuFieldReceiver {
    ident: Option<syn::Ident>,
    ty: Type,

    /// Endianness for the field
    #[darling(default)]
    endian: Option<syn::LitStr>,

    /// field bit size
    #[cfg(feature = "bits")]
    #[darling(default)]
    bits: Option<Num>,

    /// field byte size
    #[darling(default)]
    bytes: Option<Num>,

    /// tokens providing the length of the container
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    count: Result<Option<TokenStream>, ReplacementError>,

    /// tokens providing the number of bits for the length of the container
    #[cfg(feature = "bits")]
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    bits_read: Result<Option<TokenStream>, ReplacementError>,

    /// tokens providing the number of bytes for the length of the container
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    bytes_read: Result<Option<TokenStream>, ReplacementError>,

    /// a predicate to decide when to stop reading elements into the container
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    until: Result<Option<TokenStream>, ReplacementError>,

    /// read until `reader.end()`
    #[darling(default)]
    read_all: bool,

    /// apply a function to the field after it's read
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    map: Result<Option<TokenStream>, ReplacementError>,

    /// context passed to the field.
    /// A comma separated argument list.
    // TODO: The type of it should be `Punctuated<Expr, Comma>`
    //       https://github.com/TedDriggs/darling/pull/98
    #[darling(default = "default_res_opt", map = "map_option_litstr")]
    ctx: Result<Option<syn::LitStr>, ReplacementError>,

    /// map field when updating struct
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    update: Result<Option<TokenStream>, ReplacementError>,

    /// custom field reader code
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    reader: Result<Option<TokenStream>, ReplacementError>,

    /// custom field writer code
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    writer: Result<Option<TokenStream>, ReplacementError>,

    /// skip field reading/writing
    #[darling(default)]
    skip: bool,

    /// pad a number of bits before
    #[cfg(feature = "bits")]
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    pad_bits_before: Result<Option<TokenStream>, ReplacementError>,

    /// pad a number of bytes before
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    pad_bytes_before: Result<Option<TokenStream>, ReplacementError>,

    /// pad a number of bits after
    #[cfg(feature = "bits")]
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    pad_bits_after: Result<Option<TokenStream>, ReplacementError>,

    /// pad a number of bytes after
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    pad_bytes_after: Result<Option<TokenStream>, ReplacementError>,

    /// read field as temporary value, isn't stored
    #[darling(default)]
    temp: bool,

    /// write given value of temp field
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    temp_value: Result<Option<TokenStream>, ReplacementError>,

    /// default value code when used with skip
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    default: Result<Option<TokenStream>, ReplacementError>,

    /// condition to parse field
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    cond: Result<Option<TokenStream>, ReplacementError>,

    // assertion on field
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    assert: Result<Option<TokenStream>, ReplacementError>,

    // assert value of field
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    assert_eq: Result<Option<TokenStream>, ReplacementError>,

    /// seek from current position
    #[darling(default)]
    seek_rewind: bool,

    /// seek from current position
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    seek_from_current: Result<Option<TokenStream>, ReplacementError>,

    /// seek from end position
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    seek_from_end: Result<Option<TokenStream>, ReplacementError>,

    /// seek from start position
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    seek_from_start: Result<Option<TokenStream>, ReplacementError>,

    /// Bit Order of field
    #[darling(default)]
    bit_order: Option<syn::LitStr>,

    /// magic value that needs to appear before field
    #[darling(default)]
    magic: Option<syn::LitByteStr>,
}

/// Receiver for the variant-level attributes inside a enum
#[derive(Debug, FromVariant)]
#[darling(attributes(deku))]
struct DekuVariantReceiver {
    ident: syn::Ident,
    fields: ast::Fields<DekuFieldReceiver>,
    discriminant: Option<syn::Expr>,

    /// custom variant reader code
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    reader: Result<Option<TokenStream>, ReplacementError>,

    /// custom variant reader code
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    writer: Result<Option<TokenStream>, ReplacementError>,

    /// variant `id` value
    #[darling(default)]
    id: Option<Id>,

    /// variant `id_pat` value
    #[darling(default = "default_res_opt", map = "map_litstr_as_tokenstream")]
    id_pat: Result<Option<TokenStream>, ReplacementError>,

    /// variant `id` value
    #[darling(default)]
    default: Option<bool>,
}

/// Entry function for `DekuRead` proc-macro
#[proc_macro_derive(DekuRead, attributes(deku))]
pub fn proc_deku_read(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match DekuData::from_input(input.into()) {
        Ok(data) => data.emit_reader().into(),
        Err(err) => err.into(),
    }
}

/// Entry function for `DekuWrite` proc-macro
#[proc_macro_derive(DekuWrite, attributes(deku))]
pub fn proc_deku_write(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match DekuData::from_input(input.into()) {
        Ok(data) => data.emit_writer().into(),
        Err(err) => err.into(),
    }
}

/// Entry function for `DekuSize` proc-macro
#[proc_macro_derive(DekuSize, attributes(deku))]
pub fn proc_deku_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match DekuData::from_input(input.into()) {
        Ok(data) => data.emit_size().into(),
        Err(err) => err.into(),
    }
}

fn is_not_deku(attr: &syn::Attribute) -> bool {
    attr.path()
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
        syn::Fields::Unit => {}
    }
}

fn remove_temp_fields(fields: &mut syn::Fields) {
    match fields {
        syn::Fields::Named(ref mut fields) => remove_deku_temp_fields(&mut fields.named),
        syn::Fields::Unnamed(ref mut fields) => remove_deku_temp_fields(&mut fields.unnamed),
        syn::Fields::Unit => {}
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
    // Parse `deku_derive` attribute
    let nested_meta = darling::ast::NestedMeta::parse_meta_list(attr.into()).unwrap();
    let args = match DekuDerive::from_list(&nested_meta) {
        Ok(v) => v,
        Err(e) => {
            return proc_macro::TokenStream::from(e.write_errors());
        }
    };

    // Parse item
    let data = match DekuData::from_input(item.clone().into()) {
        Ok(data) => data,
        Err(err) => return err.into(),
    };

    // Generate `DekuRead` impl
    let read_impl = if args.read {
        data.emit_reader()
    } else {
        TokenStream::new()
    };

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
    let write_impl = if args.write {
        data.emit_writer()
    } else {
        TokenStream::new()
    };

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
        #read_impl

        #write_impl

        #input
    )
    .into()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use syn::parse_str;

    use super::*;

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
            #[deku(endian = "little")]
            field_c: u32,
            #[deku(endian = "big")]
            field_d: u32,
            #[deku(skip, default = "5")]
            field_e: u32,
        }"#),
        case::struct_internal_var(r#"
        struct Test {
            #[deku(bits_read = "deku::rest.len()")]
            field: Vec<u8>,
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

        let data = DekuData::from_input(parsed).unwrap();
        let res_reader = data.emit_reader_checked();
        let res_writer = data.emit_writer_checked();

        res_reader.unwrap();
        res_writer.unwrap();
    }
}
