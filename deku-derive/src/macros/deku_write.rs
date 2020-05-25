use crate::{DekuReceiver, EndianNess};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_write(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    let mut tokens = TokenStream::new();
    let ident = &input.ident;

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type")
        .fields;

    let mut field_writes = vec![];
    let mut field_overwrites = vec![];

    for (i, f) in fields.into_iter().enumerate() {
        let field_endian = f.endian.unwrap_or(input.endian);
        let field_bits = f.bits;
        let field_bytes = f.bytes;
        let field_writer = &f.writer;

        let field_len = &f.len.as_ref().map(|v| v.parse::<TokenStream>().unwrap());

        // Support named or indexed fields
        let field_ident = f
            .ident
            .as_ref()
            .map(|v| quote!(#v))
            .or_else(|| {
                Some({
                    let i = syn::Index::from(i);
                    quote!(#i)
                })
            })
            .map(|v| Some(quote! { input.#v }))
            .unwrap();

        // If `len` attr is provided, overwrite the field with the .len() of the container
        if let Some(field_len) = field_len {
            field_overwrites.push(quote! {
                // TODO: make write return a Result
                input.#field_len = #field_ident.len().try_into().unwrap();
            });
        }

        let is_le_bytes = field_endian == EndianNess::Little;

        let field_bits = match field_bits.or_else(|| field_bytes.map(|v| v * 8usize)) {
            Some(b) => quote! {Some(#b)},
            None => quote! {None},
        };

        let field_writer_func = if field_writer.is_some() {
            quote! { #field_writer }
        } else {
            quote! { field_val.write(output_is_le, field_bits) }
        };

        let field_write = quote! {
            let field_val = #field_ident;
            let output_is_le = #is_le_bytes;
            let field_bits = #field_bits;

            let bits = #field_writer_func;
            acc.extend(bits);
        };

        field_writes.push(field_write);
    }

    tokens.extend(quote! {
        impl<P> From<#ident> for BitVec<P, u8> where P: BitOrder {
            fn from(mut input: #ident) -> Self {
                use std::convert::TryInto;

                let mut acc: BitVec<P, u8> = BitVec::new();

                #(#field_overwrites)*

                #(#field_writes)*

                acc
            }
        }

        impl From<#ident> for Vec<u8> {
            fn from(mut input: #ident) -> Self {
                let mut acc: BitVec<Msb0, u8> = input.into();
                acc.into_vec()
            }
        }

        impl BitsWriter for #ident {
            fn write(self, output_is_le: bool, bit_size: Option<usize>) -> BitVec<Msb0, u8> {
                self.into()
            }
        }
    });

    //println!("{}", tokens.to_string());
    Ok(tokens)
}
