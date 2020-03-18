use crate::DekuReceiver;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_write(input: &DekuReceiver) -> TokenStream {
    let mut tokens = TokenStream::new();

    let ident = &input.ident;

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type")
        .fields;

    let field_writes = fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let field_type = &f.ty;
            let field_endian = f.endian.unwrap_or(input.endian);
            let field_bits = f.bits;
            let field_bytes = f.bytes;

            // Support named or indexed fields
            let field_ident = f
                .ident
                .as_ref()
                .map(|v| quote!(#v))
                .unwrap_or_else(|| quote!(#i));

            let field_bits = field_bits.or_else(|| field_bytes.map(|v| v * 8usize));
            let field_bits = if field_bits.is_some() {
                quote! { #field_bits }
            } else {
                quote! { #field_type::bit_size() }
            };

            let endian_flip = field_endian != input.endian;

            let field_write = quote! {
                // Reverse to write from MSB -> LSB
                for i in (0..#field_bits).rev() {
                    let field_val = if (#endian_flip) {
                        input.#field_ident.swap_bytes()
                    } else {
                        input.#field_ident
                    };

                    let bit = (field_val & 1 << i) != 0;
                    acc.push(bit)
                }
            };

            field_write
        })
        .collect::<Vec<_>>();

    tokens.extend(quote! {
        impl From<#ident> for Vec<u8> {
            fn from(input: #ident) -> Self {
                let mut acc: BitVec<Msb0, u8> = BitVec::new();

                #(#field_writes)*

                acc.into_vec()
            }
        }
    });

    tokens
}
