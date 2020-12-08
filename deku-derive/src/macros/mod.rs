use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

pub(crate) mod deku_read;
pub(crate) mod deku_write;

/// Generate enum initialization TokenStream
/// Cases:
/// - No fields: `MyEnum`
/// - Named: `MyEnum { field_idents }`
/// - Unnamed:  `MyEnum ( field_idents )`
fn gen_enum_init<V: ToTokens, I: ToTokens>(
    is_named: bool,
    enum_variant: V,
    field_idents: Vec<I>,
) -> TokenStream {
    if field_idents.is_empty() {
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
fn gen_struct_init<I: ToTokens>(is_named: bool, field_idents: Vec<I>) -> TokenStream {
    if field_idents.is_empty() {
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
    ident: I,
    field_idents: &[F],
) -> TokenStream {
    if named {
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
    let mut name = "__deku_".to_owned();
    name.push_str(&s);

    syn::Ident::new(&name, span).to_token_stream()
}

/// Map all field indents to internal idents
///
/// - Named: `{ a: __deku_a }`
/// - Unnamed: `( __deku_a )`
fn gen_internal_field_idents(named: bool, idents: Vec<&TokenStream>) -> Vec<TokenStream> {
    if named {
        idents
            .into_iter()
            .map(|i| (i.clone(), gen_internal_field_ident(i)))
            .map(|(i, h)| quote! {#i: #h})
            .collect()
    } else {
        idents.into_iter().map(gen_internal_field_ident).collect()
    }
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

/// Generate argument for `id`:
/// `#deku(endian = "big", bits = "1")` -> `Endian::Big, Size::Bits(1)`
fn gen_id_args(
    endian: Option<&syn::LitStr>,
    bits: Option<usize>,
    bytes: Option<usize>,
) -> syn::Result<TokenStream> {
    let endian = endian.map(gen_endian_from_str).transpose()?;
    let bits = bits.map(|n| quote! {deku::ctx::Size::Bits(#n)});
    let bytes = bytes.map(|n| quote! {deku::ctx::Size::Bytes(#n)});

    // FIXME: Should be `into_iter` here, see https://github.com/rust-lang/rust/issues/66145.
    let id_args = [endian.as_ref(), bits.as_ref(), bytes.as_ref()]
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
/// `#deku(endian = "big", bits = "1", ctx = "a")` -> `Endian::Big, Size::Bits(1), a`
fn gen_field_args(
    endian: Option<&syn::LitStr>,
    bits: Option<usize>,
    bytes: Option<usize>,
    ctx: Option<&Punctuated<syn::Expr, syn::token::Comma>>,
) -> syn::Result<TokenStream> {
    let endian = endian.map(gen_endian_from_str).transpose()?;
    let bits = bits.map(|n| quote! {deku::ctx::Size::Bits(#n)});
    let bytes = bytes.map(|n| quote! {deku::ctx::Size::Bytes(#n)});
    let ctx = ctx.map(|c| quote! {#c});

    // FIXME: Should be `into_iter` here, see https://github.com/rust-lang/rust/issues/66145.
    let field_args = [endian.as_ref(), bits.as_ref(), bytes.as_ref(), ctx.as_ref()]
        .iter()
        .filter_map(|i| *i)
        .collect::<Vec<_>>();

    // Because `impl DekuRead<'_, (T1, T2)>` but `impl DekuRead<'_, T1>`(not tuple)
    match &field_args[..] {
        [arg] => Ok(quote! {#arg}),
        args => Ok(quote! {#(#args),*}),
    }
}

/// Generate endian tokens from string: `big` -> `Endian::Big`.
fn gen_endian_from_str(s: &syn::LitStr) -> syn::Result<TokenStream> {
    match s.value().as_str() {
        "little" => Ok(quote! {deku::ctx::Endian::Little}),
        "big" => Ok(quote! {deku::ctx::Endian::Big}),
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
