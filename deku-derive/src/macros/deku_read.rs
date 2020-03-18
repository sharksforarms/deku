use crate::DekuReceiver;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_read(input: &DekuReceiver) -> TokenStream {
    let mut tokens = TokenStream::new();

    let ident = &input.ident;

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type")
        .fields;

    let field_reads = fields
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

            let field_read = quote! {
                #field_ident: {
                    let (ret_idx, res) = #field_type::read(idx, #field_bits);
                    let res = if (#endian_flip) {
                        res.swap_bytes()
                    } else {
                        res
                    };
                    idx = ret_idx;
                    res
                }
            };

            field_read
        })
        .collect::<Vec<_>>();

    tokens.extend(quote! {
        impl From<&[u8]> for #ident {
            fn from(input: &[u8]) -> Self {
                let mut idx = (input, 0usize);
                Self {
                    #(#field_reads),*
                }
            }
        }
    });

    tokens
}
