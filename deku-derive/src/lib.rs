//! Deku is a data-to-struct serialization/deserialization library supporting bit level granularity,
//! uses the nom crate as the consumer or “Reader” and BitVec as the “Writer”

#[macro_use]
extern crate quote;
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;

use crate::macros::deku_read;
mod helpers;
mod macros;

#[proc_macro_derive(DekuRead, attributes(deku))]
pub fn deku_read(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();

    let toks = deku_read::impl_deku_read(&ast);
    toks.into()
}

#[proc_macro_derive(DekuWrite, attributes(deku))]
pub fn deku_write(item: TokenStream) -> TokenStream {
    TokenStream::new()
}
