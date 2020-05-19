use crate::DekuReceiver;
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

    let field_writes: Result<Vec<_>, _> = fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let field_type = {
                let t = &f.ty;
                quote! { <#t> }
            };
            let field_endian = f.endian.unwrap_or(input.endian);
            let field_bits = f.bits;
            let field_bytes = f.bytes;
            let field_len = &f
                .len
                .as_ref()
                .map(|v| syn::Ident::new(&v, syn::export::Span::call_site()));

            // Support named or indexed fields
            let field_ident = f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                let ret = syn::Index::from(i);
                quote! { #ret }
            });

            if field_bits.is_some() && field_bytes.is_some() {
                return Err(darling::Error::duplicate_field(
                    "both \"bits\" and \"bytes\" specified",
                ));
            }

            let endian_flip = field_endian != input.endian;

            let field_bits = field_bits.or_else(|| field_bytes.map(|v| v * 8usize));
            let field_bits = if field_bits.is_some() {
                quote! { #field_bits }
            } else {
                quote! { #field_type::bit_size() }
            };

            let mul_len = if let Some(v) = field_len {
                quote! { * input.#v as usize }
            } else {
                quote! {}
            };

            let field_write = quote! {
                // TODO: Can this somehow be compile time?
                // Assert if we're writing more then what the type supports
                assert!(#field_bits <= #field_type::bit_size());

                let field_val = if (#endian_flip) {
                    input.#field_ident.swap_endian()
                } else {
                    input.#field_ident
                };

                let field_bytes = field_val.write();

                let mut bits: BitVec<P, u8> = field_bytes.into();
                let index = bits.len() - #field_bits #mul_len;
                acc.extend_from_slice(&bits.as_bitslice()[index..]);
            };

            Ok(field_write)
        })
        .collect();

    let field_writes = field_writes?;

    tokens.extend(quote! {
        impl<P> From<#ident> for BitVec<P, u8> where P: BitOrder {
            fn from(input: #ident) -> Self {
                let mut acc: BitVec<P, u8> = BitVec::new();

                #(#field_writes)*

                acc
            }
        }

        impl From<#ident> for Vec<u8> {
            fn from(input: #ident) -> Self {
                let mut acc: BitVec<Msb0, u8> = input.into();

                // pad to next byte
                let pad_amt = 8 * ((acc.len() + 7) / 8) - acc.len();
                for _i in 0..pad_amt {
                    acc.insert(0, false);
                }

                acc.into_vec()
            }
        }

        impl BitsWriter for #ident {
            fn write(self) -> Vec<u8> {
                self.into()
            }

            fn swap_endian(self) -> Self {
                // do nothing
                self
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}
