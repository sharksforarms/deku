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

    let mut field_writes = vec![];
    let mut field_inputs = vec![];

    for (i, f) in fields.into_iter().enumerate() {
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
        let (field_ident_orig, field_ident) = f
            .ident
            .as_ref()
            .map(|v| quote!(#v))
            .or_else(|| {
                Some({
                    let i = syn::Index::from(i);
                    quote!(#i)
                })
            })
            .and_then(|v| {
                let field_ident = syn::Ident::new(
                    &format!("field_{}", quote! { #v }),
                    syn::export::Span::call_site(),
                );

                Some((quote! {#v}, quote! { #field_ident }))
            })
            .unwrap();

        // If `len` attr is provided, overwrite the field with the .len() of the container
        let field_input_len_overwrite = field_len.as_ref().map(|v| {
            let len_field_ident = syn::Ident::new(
                &format!("field_{}", quote! { #v }), // TODO: duplicate code
                syn::export::Span::call_site(),
            );

            quote! {
                let #len_field_ident = #field_ident.len();
            }
        });

        let field_input = quote! {
            let #field_ident = input.#field_ident_orig;

            #field_input_len_overwrite
        };
        field_inputs.push(field_input);

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
            let len_field_ident = syn::Ident::new(
                &format!("field_{}", quote! { #v }), // TODO: duplicate code
                syn::export::Span::call_site(),
            );
            quote! { * #len_field_ident as usize }
        } else {
            quote! {}
        };

        let field_write = quote! {
            // TODO: Can this somehow be compile time?
            // Assert if we're writing more then what the type supports
            assert!(#field_bits <= #field_type::bit_size());

            let field_val = if (#endian_flip) {
                #field_ident.swap_endian()
            } else {
                #field_ident
            };

            let bits = field_val.write();
            let index = bits.len() - #field_bits #mul_len;
            acc.extend_from_slice(&bits.as_bitslice()[index..]);
        };

        field_writes.push(field_write);
    }

    tokens.extend(quote! {
        impl<P> From<#ident> for BitVec<P, u8> where P: BitOrder {
            fn from(input: #ident) -> Self {
                let mut acc: BitVec<P, u8> = BitVec::new();

                #(#field_inputs)*

                #(#field_writes)*

                acc
            }
        }

        impl From<#ident> for Vec<u8> {
            fn from(input: #ident) -> Self {
                let mut acc: BitVec<Msb0, u8> = input.into();
                acc.into_vec()
            }
        }

        impl BitsWriter for #ident {
            fn write(self) -> BitVec<Msb0, u8> {
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
