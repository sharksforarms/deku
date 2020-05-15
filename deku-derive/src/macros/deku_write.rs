use crate::DekuReceiver;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};

pub(crate) fn emit_deku_write(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    let mut tokens = TokenStream::new();
    let mut seen_field_names: HashSet<String> = HashSet::new();
    let mut vec_len_fields = HashMap::new();
    let ident = &input.ident;

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type")
        .fields;

    // Extract fields that represent Vec lenghts
    for (i, f) in fields.iter().enumerate() {
        if let Some(ref field_len_name) = f.vec_len {
            vec_len_fields.insert(
                field_len_name,
                f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                    let ret = syn::Index::from(i);
                    quote! { #ret }
                }),
            );
        }
    }

    let field_writes: Result<Vec<_>, _> = fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let mut field_type = &f.ty;
            let field_endian = f.endian.unwrap_or(input.endian);
            let field_bits = f.bits;
            let field_bytes = f.bytes;
            let field_vec_len = &f.vec_len;
            let mut vec_ident = None;

            // Support named or indexed fields
            let mut field_ident = f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                let ret = syn::Index::from(i);
                quote! { #ret }
            });
            let field_name = field_ident.to_string();

            if field_bits.is_some() && field_bytes.is_some() {
                return Err(darling::Error::duplicate_field(
                    "both \"bits\" and \"bytes\" specified",
                ));
            }

            let endian_flip = field_endian != input.endian;

            if let Some(len_field_name) = field_vec_len {
                vec_ident = Some(super::validate_vec_len(
                    &mut field_ident,
                    &mut field_type,
                    len_field_name,
                    &mut seen_field_names,
                )?);
            } else {
                seen_field_names.insert(field_ident.to_string());

                // If this field contains the length of a vec
                if let Some(vec_ident) = vec_len_fields.get(&field_name) {
                    field_ident = quote! {
                        input.#vec_ident.len()
                    };
                } else {
                    field_ident = quote! {
                        input.#field_ident
                    };
                }
            }

            let field_bits = field_bits.or_else(|| field_bytes.map(|v| v * 8usize));
            let field_bits = if field_bits.is_some() {
                quote! { #field_bits }
            } else {
                quote! { #field_type::bit_size() }
            };

            let mut field_write = quote! {
                // TODO: Can this somehow be compile time?
                // Assert if we're writing more then what the type supports
                assert!(#field_bits <= #field_type::bit_size());

                let field_val = if (#endian_flip) {
                    #field_ident
                } else {
                    #field_ident.swap_endian()
                };

                let field_bytes = field_val.write();

                // Reverse to write from MSB -> LSB
                for i in (0..#field_bits).rev() {
                    let field_val = field_bytes[i/8];
                    let bit = (field_val & 1 << (i%8)) != 0;
                    acc.push(bit)
                }
            };

            // if this is a vec of field_type
            if let Some(vec_name) = vec_ident {
                field_write = quote! {
                    for #field_ident in input.#vec_name.iter() {
                        let #field_ident = *#field_ident;
                        #field_write
                    }
                };
            }
            //println!("{}", field_write.to_string());

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
                let acc: BitVec<Msb0, u8> = input.into();
                acc.into_vec()
            }
        }

        impl BitsWriter for #ident {
            fn write(self) -> Vec<u8> {
                // TODO: This could be improved I think.

                // Accumulate the result for the struct and reverse the bits
                let mut acc: BitVec::<Lsb0, u8> = self.into();
                let bs = acc.as_mut_bitslice();
                bs[..].reverse();

                acc.into_vec()
            }

            fn swap_endian(self) -> Self {
                // do nothing
                self
            }
        }
    });

    Ok(tokens)
}
