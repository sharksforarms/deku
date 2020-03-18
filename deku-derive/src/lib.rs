//! Deku is a data-to-struct serialization/deserialization library supporting bit level granularity,
//! uses the nom crate as the consumer or “Reader” and BitVec as the “Writer”

use darling::ast;
use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;
use proc_macro2::TokenStream;

mod macros;
use crate::macros::{deku_read::emit_deku_read, deku_write::emit_deku_write};

#[derive(Debug, Clone, Copy, PartialEq, FromMeta)]
#[darling(default)]
enum EndianNess {
    Little,
    Big,
}

impl Default for EndianNess {
    fn default() -> Self {
        #[cfg(target_endian = "little")]
        let ret = EndianNess::Little;

        #[cfg(target_endian = "big")]
        let ret = EndianNess::Big;

        ret
    }
}

#[derive(Debug, FromDeriveInput)]
// Process all `deku` attributes and only support structs
#[darling(attributes(deku), supports(struct_any))]
struct DekuReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), DekuFieldReceiver>,

    // Default EndianNess
    #[darling(default)]
    endian: EndianNess,
}

impl DekuReceiver {
    fn emit_reader(&self) -> TokenStream {
        emit_deku_read(self)
    }

    fn emit_writer(&self) -> TokenStream {
        emit_deku_write(self)
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(deku))]
struct DekuFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    endian: Option<EndianNess>,

    #[darling(default)]
    bits: Option<usize>,

    #[darling(default)]
    bytes: Option<usize>,
}

#[proc_macro_derive(DekuRead, attributes(deku))]
pub fn proc_deku_read(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let receiver = DekuReceiver::from_derive_input(&syn::parse(input).unwrap()).unwrap();
    let tokens = receiver.emit_reader();
    tokens.into()
}

#[proc_macro_derive(DekuWrite, attributes(deku))]
pub fn proc_deku_write(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let receiver = DekuReceiver::from_derive_input(&syn::parse(input).unwrap()).unwrap();
    let tokens = receiver.emit_writer();
    tokens.into()
}
