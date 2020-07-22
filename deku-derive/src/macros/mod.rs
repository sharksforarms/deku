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
/// Named: `#ident { ref fields }`
/// Unnamed: `#ident ( ref fields )`
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

fn gen_hidden_field_ident(ident: TokenStream) -> TokenStream {

    // We can't concat to token, so I use string.
    // See https://github.com/rust-lang/rust/issues/29599
    let span = ident.span();
    let s = ident.to_string();
    let mut name = "__".to_owned();
    name.push_str(&s);

    syn::Ident::new(&name, span).to_token_stream()
}

/// -> `{ a: __a }` or `(__a)`
fn gen_hidden_field_idents(named: bool, idents: Vec<TokenStream>) -> Vec<TokenStream> {
    // -> `{ a: __a }` or `(__a)`
    if named {
        idents
            .into_iter()
            .map(|i| (i.clone(), gen_hidden_field_ident(i)))
            .map(|(i, h)| quote! {#i: #h})
            .collect()
    } else {
        idents
            .into_iter()
            .map(gen_hidden_field_ident)
            .collect()
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

fn gen_ctx_types_and_arg(
    ctx: Option<&Punctuated<syn::FnArg, syn::token::Comma>>,
) -> syn::Result<(TokenStream, TokenStream)> {
    if let Some(ctx) = ctx {
        let pats_types = split_ctx_to_pats_and_types(ctx)?;

        if pats_types.len() == 1 {
            // remove paren for single item
            let (pat, ty) = pats_types[0];
            Ok((quote! {#ty}, quote! {#pat:#ty}))
        } else {
            let pats = pats_types.iter().map(|(pat, _)| pat);
            let types = pats_types.iter().map(|(_, ty)| ty);

            // avoid move
            let types2 = types.clone();

            // "a: u8, b: usize" -> (u8, usize)
            let ctx_types = quote! {(#(#types2),*)};
            // "a: u8, b: usize" -> (a, b): (u8, usize)
            let ctx_arg = quote! {(#(#pats),*): (#(#types),*)};

            Ok((ctx_types, ctx_arg))
        }
    } else {
        Ok((quote! {()}, quote! {_: ()}))
    }
}
