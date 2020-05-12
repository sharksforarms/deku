use crate::DekuReceiver;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_read(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    let mut tokens = TokenStream::new();

    let ident = &input.ident;

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type")
        .fields;

    let mut field_reads = vec![];
    let mut field_bit_sizes = vec![];

    // Iterate each field, creating tokens for implementations
    for (i, f) in fields.into_iter().enumerate() {
        let field_type = &f.ty;
        let field_endian = f.endian.unwrap_or(input.endian);
        let field_bits = f.bits;
        let field_bytes = f.bytes;

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

        let field_bits = field_bits.or_else(|| field_bytes.map(|v| v * 8usize));
        let field_bits = if field_bits.is_some() {
            quote! { #field_bits }
        } else {
            quote! { #field_type::bit_size() }
        };

        let endian_flip = field_endian != input.endian;

        // Create field read token for TryFrom trait
        let field_read = quote! {
            #field_ident: {
                // TODO: Can this somehow be compile time?
                assert!(#field_bits <= #field_type::bit_size());

                let (rest, value) = #field_type::read(input, #field_bits)?;
                let value = if (#endian_flip) {
                    value.swap_bytes()
                } else {
                    value
                };

                input = rest;

                value
            }
        };

        field_reads.push(field_read);

        // Create bit size token for BitSize trait
        let field_bit_size = quote! {
            #field_bits
        };

        field_bit_sizes.push(field_bit_size);
    }

    tokens.extend(quote! {
        impl TryFrom<&[u8]> for #ident {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
                let mut input = input.bits::<Msb0>();

                if input.len() < #ident::bit_size() {
                    return Err(DekuError::Parse(format!("not enough data: expected {} got {}", #ident::bit_size(), input.len())));
                }

                if input.len() > #ident::bit_size() {
                    return Err(DekuError::Parse(format!("too much data: expected {} got {}", #ident::bit_size(), input.len())));
                }

                let res = Ok(Self {
                    #(#field_reads),*
                });

                if !input.is_empty() {
                    unreachable!();
                }


                res
            }
        }

        impl #ident {
            fn swap_bytes(self) -> Self {
                self
            }
        }
        impl BitsSize for #ident {
            fn bit_size() -> usize {
                #(#field_bit_sizes)+*
            }
        }

        impl BitsReader for #ident {
            fn read(input: &BitSlice<Msb0, u8>, len: usize) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let (bits, rest) = input.split_at(len);
                let mut input = bits;
                let res = Self {
                    #(#field_reads),*
                };

                Ok((rest, res))
            }
        }
    });

    Ok(tokens)
}
