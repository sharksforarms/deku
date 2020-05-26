use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub(crate) mod deku_read;
pub(crate) mod deku_write;

fn wrap_turbofish<T: ToTokens>(input: T) -> TokenStream {
    quote! { <#input> }
}

fn option_as_literal_token<T: ToTokens>(input: Option<T>) -> TokenStream {
    match input {
        Some(b) => quote! {Some(#b)},
        None => quote! {None},
    }
}

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

fn gen_struct_init<I: ToTokens>(is_named: bool, field_idents: Vec<I>) -> TokenStream {
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
