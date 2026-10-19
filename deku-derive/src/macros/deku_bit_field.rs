use darling::ast::Data;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;

use super::{low_mask, no_variant_error, unsigned_width};
use crate::{DekuData, Id, Num};

/// `DekuBitField` for an enum read by reading its id and nothing else.
///
/// Anything that needs the reader itself is a compile error rather than a
/// missing impl, so the enum's author learns here, not at a struct using it.
pub(crate) fn emit_deku_bit_field(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();
    let ident = &input.ident;
    let ident_as_string = ident.to_string();

    let Data::Enum(variants) = &input.data else {
        return Err(error(ident.span(), "only enums are supported"));
    };
    if !input.generics.params.is_empty() {
        return Err(error(
            input.generics.span(),
            "generic enums are not supported",
        ));
    }
    // With `id` the discriminant comes from context, not off the wire.
    if let Some(id) = &input.id {
        return Err(error(
            id.to_token_stream().span(),
            "`id` takes the discriminant from context, so there are no bits to convert",
        ));
    }
    if input.any_container_set_incompatible_with_bit_field() {
        return Err(error(
            ident.span(),
            "`magic`, `bytes` and `seek_*` need the reader, not just the id bits",
        ));
    }
    // `Msb0` is the default, so absent is fine; "lsb" or a ctx order is not.
    if let Some(order) = &input.bit_order
        && order.value() != "msb"
    {
        return Err(error(order.span(), "only `Msb0` lines up with a bit run"));
    }

    let Some(id_type) = &input.id_type else {
        return Err(error(ident.span(), "`id_type` is required"));
    };
    let Some(width) = unsigned_width(&id_type.to_string()) else {
        return Err(error(
            id_type.span(),
            "`id_type` must be `u8`, `u16`, `u32` or `u64`",
        ));
    };
    let bits = match &input.bits {
        Some(Num::LitInt(lit)) => {
            let bits = lit.base10_parse::<usize>()?;
            if bits == 0 || bits > width {
                return Err(error(
                    lit.span(),
                    "`bits` must be between 1 and the width of `id_type`",
                ));
            }
            bits
        }
        Some(Num::TokenStream(ts)) => {
            return Err(error(ts.span(), "`bits` must be an integer literal"));
        }
        None => width,
    };
    // Past a byte, only big-endian lines up with a run. An endian from context
    // is fine, since runs only form in big-endian structs.
    if bits > 8 {
        let endian = input.id_endian.as_ref().or(input.endian.as_ref());
        if endian.is_none_or(|e| e.value() == "little") {
            return Err(error(
                ident.span(),
                "an id wider than 8 bits must be big-endian, or take its endian from context",
            ));
        }
    }

    let mut from_arms = Vec::with_capacity(variants.len());
    let mut to_arms = Vec::with_capacity(variants.len());
    for variant in variants {
        if variant.any_variant_set_incompatible_with_bit_field() {
            return Err(error(
                variant.ident.span(),
                "every variant must be a unit variant, without `id_pat`, `reader`, `writer` or `default`",
            ));
        }
        // `id = 1` and `id = "1"` are the same literal, written two ways.
        let id = match &variant.id {
            Some(Id::Int(id)) => id.clone(),
            Some(Id::TokenStream(ts)) => match syn::parse2::<syn::LitInt>(ts.clone()) {
                Ok(id) => id,
                Err(_) => return Err(error(ts.span(), ID_MUST_BE_AN_INTEGER)),
            },
            _ => return Err(error(variant.ident.span(), ID_MUST_BE_AN_INTEGER)),
        };
        // The individual write would reject it at run time and no read could
        // ever produce it, so it can only be a mistake.
        let value = id.base10_parse::<u64>()?;
        if bits < 64 && value >> bits != 0 {
            return Err(error(
                id.span(),
                &format!("`id` {id} does not fit in {bits} bits"),
            ));
        }
        let variant_ident = &variant.ident;
        from_arms.push(quote! { #id => Self::#variant_ident });
        to_arms.push(quote! { Self::#variant_ident => #id });
    }

    let mask = low_mask(bits);
    let no_variant = no_variant_error(&crate_, &ident_as_string);

    Ok(quote! {
        #[automatically_derived]
        impl ::#crate_::DekuBitField for #ident {
            const BITS: usize = #bits;

            #[inline]
            fn from_bit_run(__deku_raw: u64) -> core::result::Result<Self, ::#crate_::DekuError> {
                let __deku_variant_id = (__deku_raw & #mask) as #id_type;
                Ok(match __deku_variant_id {
                    #(#from_arms,)*
                    _ => return Err(#no_variant),
                })
            }

            #[inline]
            fn to_bit_run(&self) -> core::result::Result<u64, ::#crate_::DekuError> {
                let __deku_id: #id_type = match self {
                    #(#to_arms,)*
                };
                Ok(__deku_id as u64)
            }
        }
    })
}

const ID_MUST_BE_AN_INTEGER: &str =
    "every variant needs an integer `id`, such as `id = 1` or `id = \"0x01\"`";

fn error(span: Span, msg: &str) -> syn::Error {
    syn::Error::new(span, format!("DekuBitField: {msg}"))
}
