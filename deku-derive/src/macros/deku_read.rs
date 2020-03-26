use crate::DekuReceiver;
use darling;
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

    let field_reads: Result<Vec<_>, _> = fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
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

            let field_read = quote! {
                #field_ident: {
                    let res = #field_type::read(idx, &mut bit_index, #field_bits)?;
                    let res = if (#endian_flip) {
                        res.swap_bytes()
                    } else {
                        res
                    };
                    res
                }
            };

            Ok(field_read)
        })
        .collect();

    let field_reads = field_reads?;

    tokens.extend(quote! {
        impl TryFrom<&[u8]> for #ident {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
                let mut bit_index = 0usize;
                let idx = input.bits::<Msb0>();
                Ok(Self {
                    #(#field_reads),*
                })
            }
        }
    });

    Ok(tokens)
}
