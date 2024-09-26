use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Lifetime, LitStr};

use crate::Num;

pub(crate) mod deku_read;
pub(crate) mod deku_write;

#[cfg(feature = "proc-macro-crate")]
fn get_crate_name() -> Ident {
    let found_crate =
        proc_macro_crate::crate_name("deku").unwrap_or(proc_macro_crate::FoundCrate::Itself);

    let crate_name = match found_crate {
        proc_macro_crate::FoundCrate::Itself => "deku".to_string(),
        proc_macro_crate::FoundCrate::Name(name) => name,
    };

    Ident::new(&crate_name, Span::call_site())
}

// proc-macro-crate depends on std, for no_std, use default name. Sorry.
#[cfg(not(feature = "proc-macro-crate"))]
fn get_crate_name() -> Ident {
    Ident::new("deku", Span::call_site())
}

/// Generate enum initialization TokenStream
/// Cases:
/// - No fields: `MyEnum`
/// - Named: `MyEnum { field_idents }`
/// - Unnamed:  `MyEnum ( field_idents )`
fn gen_enum_init<V: ToTokens, I: ToTokens>(
    is_named: bool,
    enum_variant: V,
    field_idents: impl Iterator<Item = I>,
) -> TokenStream {
    let mut field_idents = field_idents.peekable();
    if field_idents.peek().is_none() {
        return quote! { #enum_variant };
    }

    if is_named {
        quote! {
            #enum_variant {
                #(#field_idents),*
            }
        }
    } else {
        quote! {
            #enum_variant (
                #(#field_idents),*
            )
        }
    }
}

/// Generate struct initialization TokenStream
/// Cases:
/// - No fields: `Self {}`
/// - Named: `Self { field_idents }`
/// - Unnamed:  `Self ( field_idents )`
fn gen_struct_init<I: ToTokens>(
    is_named: bool,
    field_idents: impl Iterator<Item = I>,
) -> TokenStream {
    let mut field_idents = field_idents.peekable();
    if field_idents.peek().is_none() {
        return quote! { Self {} };
    }

    if is_named {
        quote! {
            Self {
                #(#field_idents),*
            }
        }
    } else {
        quote! {
            Self (
                #(#field_idents),*
            )
        }
    }
}

/// Generate struct destruction
///
/// - Named: `#ident { ref fields }`
/// - Unnamed: `#ident ( ref fields )`
fn gen_struct_destruction<I: ToTokens, F: ToTokens>(
    named: bool,
    unit: bool,
    ident: I,
    field_idents: impl Iterator<Item = F>,
) -> TokenStream {
    if unit {
        quote! {
            #ident
        }
    } else if named {
        quote! {
            #ident {
                #(ref #field_idents),*
            }
        }
    } else {
        quote! {
            #ident (
                #(ref #field_idents),*
            )
        }
    }
}

/// Convert a field ident to internal ident:
/// `a` -> `__deku_a`
fn gen_internal_field_ident(ident: &TokenStream) -> TokenStream {
    // Concat token: https://github.com/rust-lang/rust/issues/29599
    let span = ident.span();
    let s = ident.to_string();
    let mut name = "__deku___".to_owned();
    // If its a raw identifier, we must remove 'r#'
    name.push_str(s.strip_prefix("r#").unwrap_or(&s));

    syn::Ident::new(&name, span).to_token_stream()
}

/// Map all field indents to internal idents
///
/// - Named: `{ a: __deku_a }`
/// - Unnamed: `( __deku_a )`
fn gen_internal_field_idents<'a>(
    named: bool,
    idents: impl Iterator<Item = &'a TokenStream> + 'a,
) -> impl Iterator<Item = TokenStream> + 'a {
    idents.map(move |i| {
        if named {
            let h = gen_internal_field_ident(i);
            quote! {#i: #h}
        } else {
            gen_internal_field_ident(i)
        }
    })
}

fn split_ctx_to_pats_and_types(
    ctx: &Punctuated<syn::FnArg, syn::token::Comma>,
) -> syn::Result<Vec<(&syn::Pat, &syn::Type)>> {
    // `()` or `(u8, u32)`
    ctx.iter()
        .map(|arg| {
            match arg {
                syn::FnArg::Typed(pat_type) => Ok((pat_type.pat.as_ref(), pat_type.ty.as_ref())),
                // a self is unacceptable
                syn::FnArg::Receiver(r) => Err(syn::Error::new(r.span(), "Unacceptable context")),
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

/// Generate ctx types and argument
///
/// - Empty: arg: `(): ()`, type: `()`
/// - One: arg: `a: usize`, type: `usize`
/// - Other: arg: `(a, b, ...): (u8, u8, ...)`, type: `(u8, u8, ...)`
fn gen_ctx_types_and_arg(
    ctx: Option<&Punctuated<syn::FnArg, syn::token::Comma>>,
) -> syn::Result<(TokenStream, TokenStream)> {
    if let Some(ctx) = ctx {
        let pats_types = split_ctx_to_pats_and_types(ctx)?;

        if pats_types.len() == 1 {
            // remove parens for single item
            let (pat, ty) = pats_types[0];
            Ok((quote! {#ty}, quote! {#pat:#ty}))
        } else {
            let pats = pats_types.iter().map(|(pat, _)| pat);
            let types = pats_types.iter().map(|(_, ty)| ty);

            // "a: u8, b: usize" -> (u8, usize)
            let types_cpy = types.clone();
            let ctx_types = quote! {(#(#types_cpy),*)};
            // "a: u8, b: usize" -> (a, b): (u8, usize)
            let ctx_arg = quote! {(#(#pats),*): (#(#types),*)};

            Ok((ctx_types, ctx_arg))
        }
    } else {
        Ok((quote! {()}, quote! {_: ()}))
    }
}

/// Generate type from matching ident from `id` in `ctx`
///
/// - #[deku(ctx = "test: u16, my_id: u8", id = "my_id")], will return `u8`
/// - #[deku(ctx = "test: u16, my_id: u8", id = "my_id, test")], will return `u8, u16`
fn gen_type_from_ctx_id(
    ctx: &Punctuated<syn::FnArg, syn::token::Comma>,
    id: &crate::Id,
) -> Option<TokenStream> {
    let parser = Punctuated::<Ident, Comma>::parse_terminated;
    let Ok(s) = parser.parse(id.to_token_stream().into()) else {
        return None;
    };
    let mut matching_types = quote! {};
    for s in s {
        let id = syn::Ident::new(&s.to_string(), id.span());

        let types = ctx.iter().find_map(|arg| {
            let mut t = None;
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    if id == ident.ident {
                        let mut pat_type = pat_type.clone();
                        if let syn::Type::Reference(r) = pat_type.ty.as_mut() {
                            r.lifetime = Some(Lifetime::new("'__deku", Span::call_site()));
                        }
                        let ty = &pat_type.ty;
                        t = Some(quote! {#ty});
                    }
                }
            }

            t
        });
        if matching_types.is_empty() {
            matching_types = quote! {#matching_types #types};
        } else {
            matching_types = quote! {#matching_types, #types};
        }
    }

    if matching_types.is_empty() {
        None
    } else {
        Some(matching_types)
    }
}

/// Generate argument for `id`:
/// `#deku(endian = "big", bits = 1)` -> `Endian::Big, BitSize(1)`
/// `#deku(endian = "big", bytes = 1)` -> `Endian::Big, ByteSize(1)`
pub(crate) fn gen_id_args(
    endian: Option<&syn::LitStr>,
    id_endian: Option<&syn::LitStr>,
    bits: Option<&Num>,
    bytes: Option<&Num>,
    bit_order: Option<&syn::LitStr>,
) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    let endian = id_endian
        .map(gen_endian_from_str)
        .or_else(|| endian.map(gen_endian_from_str))
        .transpose()?;
    let bits = bits.map(|n| quote! {::#crate_::ctx::BitSize(#n)});
    let bytes = bytes.map(|n| quote! {::#crate_::ctx::ByteSize(#n)});
    let bit_order = bit_order.map(gen_bit_order_from_str).transpose()?;

    // FIXME: Should be `into_iter` here, see https://github.com/rust-lang/rust/issues/66145.
    let id_args = [
        endian.as_ref(),
        bits.as_ref(),
        bytes.as_ref(),
        bit_order.as_ref(),
    ]
    .iter()
    .filter_map(|i| *i)
    .collect::<Vec<_>>();

    match &id_args[..] {
        [arg] => Ok(quote! {#arg}),
        args => Ok(quote! {#(#args),*}),
    }
}

/// Generate argument for fields:
///
/// `#deku(endian = "big", bits = 1, ctx = "a")` -> `Endian::Big, BitSize(1), a`
/// `#deku(endian = "big", bytes = 1, ctx = "a")` -> `Endian::Big, ByteSize(1), a`
fn gen_field_args(
    endian: Option<&syn::LitStr>,
    bits: Option<&Num>,
    bytes: Option<&Num>,
    ctx: Option<&Punctuated<syn::Expr, syn::token::Comma>>,
    bit_order: Option<&syn::LitStr>,
) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    let endian = endian.map(gen_endian_from_str).transpose()?;
    let bits = bits.map(|n| quote! {::#crate_::ctx::BitSize(#n)});
    let bytes = bytes.map(|n| quote! {::#crate_::ctx::ByteSize(#n)});
    let bit_order = bit_order.map(gen_bit_order_from_str).transpose()?;
    let ctx = ctx.map(|c| quote! {#c});

    // FIXME: Should be `into_iter` here, see https://github.com/rust-lang/rust/issues/66145.
    // TODO: the order here should be documented
    let field_args = [
        endian.as_ref(),
        bits.as_ref(),
        bytes.as_ref(),
        bit_order.as_ref(),
        ctx.as_ref(),
    ]
    .iter()
    .filter_map(|i| *i)
    .collect::<Vec<_>>();

    // Because `impl DekuRead<'_, (T1, T2)>` but `impl DekuRead<'_, T1>`(not tuple)
    match &field_args[..] {
        [arg] => Ok(quote! {#arg}),
        args => Ok(quote! {#(#args),*}),
    }
}

/// Generate bit_order tokens from string: `lsb` -> `Order::Lsb0`.
fn gen_bit_order_from_str(s: &syn::LitStr) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    match s.value().as_str() {
        "lsb" => Ok(quote! {::#crate_::ctx::Order::Lsb0}),
        "msb" => Ok(quote! {::#crate_::ctx::Order::Msb0}),
        _ => {
            // treat as variable, possibly from `ctx`
            let v: TokenStream = s.value().parse()?;
            Ok(quote! {#v})
        }
    }
}

/// Generate endian tokens from string: `big` -> `Endian::Big`.
fn gen_endian_from_str(s: &syn::LitStr) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    match s.value().as_str() {
        "little" => Ok(quote! {::#crate_::ctx::Endian::Little}),
        "big" => Ok(quote! {::#crate_::ctx::Endian::Big}),
        _ => {
            // treat as variable, possibly from `ctx`
            let v: TokenStream = s.value().parse()?;
            Ok(quote! {#v})
        }
    }
}

/// Wraps a TokenStream with a closure providing access to `ctx` variables when
/// `ctx_default` is provided
fn wrap_default_ctx(
    body: TokenStream,
    ctx: &Option<syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>>,
    ctx_default: &Option<Punctuated<syn::Expr, syn::token::Comma>>,
) -> TokenStream {
    if let (Some(ctx), Some(ctx_default)) = (ctx, ctx_default) {
        // wrap in a function to make `ctx` variables in scope
        quote! {
            |#ctx| -> Result<_, _> {
                #body
            }(#ctx_default)
        }
    } else {
        body
    }
}

/// Returns true if the literal substring `s` is in the token
fn token_contains_string(tok: &Option<TokenStream>, s: &str) -> bool {
    tok.as_ref()
        .map(|v| {
            let v = v.to_string();
            v.contains(s)
        })
        .unwrap_or(false)
}

#[cfg(feature = "bits")]
fn pad_bits(
    bits: Option<&TokenStream>,
    bytes: Option<&TokenStream>,
    bit_order: Option<&LitStr>,
    emit_padding: fn(&TokenStream, bit_order: Option<&LitStr>) -> TokenStream,
) -> TokenStream {
    match (bits, bytes) {
        (Some(pad_bits), Some(pad_bytes)) => {
            emit_padding(&quote! { (#pad_bits) + ((#pad_bytes) * 8) }, bit_order)
        }
        (Some(pad_bits), None) => emit_padding(pad_bits, bit_order),
        (None, Some(pad_bytes)) => emit_padding(&quote! {((#pad_bytes) * 8)}, bit_order),
        (None, None) => quote!(),
    }
}

#[cfg(not(feature = "bits"))]
fn pad_bytes(
    bytes: Option<&TokenStream>,
    emit_padding: fn(&TokenStream) -> TokenStream,
) -> TokenStream {
    match bytes {
        Some(pad_bytes) => emit_padding(&quote! {((#pad_bytes))}),
        None => quote!(),
    }
}

/// assertion is false, raise error
fn assertion_failed(
    v: &TokenStream,
    ident: &str,
    field_ident_str: &str,
    field_ident: Option<&TokenStream>,
) -> TokenStream {
    let crate_ = get_crate_name();
    let stringify = if let Some(field_ident) = field_ident {
        quote! { stringify!(#field_ident == #v) }
    } else {
        quote! { stringify!(#v) }
    };
    #[cfg(feature = "no-assert-string")]
    {
        quote! {
            return Err(::#crate_::DekuError::AssertionNoStr);
        }
    }
    #[cfg(not(feature = "no-assert-string"))]
    {
        quote! {
            extern crate alloc;
            use alloc::borrow::Cow;
            return Err(::#crate_::DekuError::Assertion(Cow::from(format!(
                "{}.{} field failed assertion: {}",
                #ident,
                #field_ident_str,
                #stringify,
            ))));
        }
    }
}
