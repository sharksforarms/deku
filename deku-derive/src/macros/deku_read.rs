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

    // check if the first field has an ident, if not, it's a unnamed struct
    let is_unnamed_struct = fields.get(0).and_then(|v| v.ident.as_ref()).is_none();

    let mut field_variables = vec![];
    let mut field_idents = vec![];
    let mut field_bit_sizes = vec![];

    // Iterate each field, creating tokens for implementations
    for (i, f) in fields.into_iter().enumerate() {
        let field_type = {
            let t = &f.ty;
            quote! { <#t> }
        };

        let field_endian = f.endian.unwrap_or(input.endian);
        let field_bits = f.bits;
        let field_bytes = f.bytes;
        let field_reader = &f.reader;
        let field_len = &f
            .len
            .as_ref()
            .map(|v| syn::Ident::new(&v, syn::export::Span::call_site()));

        // Holds the generated code to read into a field
        let field_reader = field_reader.as_ref().map(|fn_str| {
            let fn_ident: TokenStream = fn_str.parse().unwrap();

            // TODO: Assert the shape of fn_ident? Only allow a structured function call instead of anything?

            quote! { #fn_ident; }
        });

        // Support named or indexed fields
        let field_ident = f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
            let index = syn::Index::from(i);
            let field_ident = syn::Ident::new(
                &format!("field_{}", quote! { #index }),
                syn::export::Span::call_site(),
            );

            quote! { #field_ident }
        });

        field_idents.push(field_ident.clone());

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

        let field_read_func = if field_reader.is_some() {
            quote! { #field_reader }
        } else if field_len.is_some() {
            quote! { #field_type::read(rest, field_bits, #field_len as usize) }
        } else {
            quote! { #field_type::read(rest, field_bits) }
        };

        // Create field read token for TryFrom trait
        let field_read = quote! {
            let #field_ident = {
                let field_bits = #field_bits;

                // TODO: Can this somehow be compile time?
                assert!(field_bits <= #field_type::bit_size());

                let read_ret = #field_read_func;
                let (new_rest, value) = read_ret?;

                let value = if (#endian_flip) {
                    value.swap_endian()
                } else {
                    value
                };

                rest = new_rest;

                value
            };
        };

        field_variables.push(field_read);

        // Create bit size token for BitSize trait
        let field_bit_size = quote! {
            #field_bits
        };

        field_bit_sizes.push(field_bit_size);
    }

    let initialize_struct = if is_unnamed_struct {
        quote! {
            Self (
                #(#field_idents),*
            )
        }
    } else {
        quote! {
            Self {
                #(#field_idents),*
            }
        }
    };

    tokens.extend(quote! {
        impl TryFrom<&[u8]> for #ident {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {

                // TODO: if there's a dynamic type in the struct, then we're not able to check ahead of time
                // let input_bits = input.len() * 8;
                // if input_bits > #ident::bit_size() {
                //     return Err(DekuError::Parse(format!("too much data: expected {} got {}", #ident::bit_size(), input_bits)));
                // }

                let (rest, res) = Self::from_bytes(input)?;

                // TODO: This should always be empty due to the check above
                // if !rest.is_empty() {
                //     unreachable!();
                // }

                Ok(res)
            }
        }

        impl #ident {
            fn from_bytes(input: &[u8]) -> Result<(&[u8], Self), DekuError> {
                let mut rest = input.bits::<Msb0>();

                // TODO: if there's a dynamic type in the struct, then we're not able to check ahead of time
                // if rest.len() < #ident::bit_size() {
                //     return Err(DekuError::Parse(format!("not enough data: expected {} got {}", #ident::bit_size(), rest.len())));
                // }

                #(#field_variables)*
                let value = #initialize_struct;

                Ok((rest.as_slice(), value))
            }
        }

        impl BitsSize for #ident {
            fn bit_size() -> usize {
                #(#field_bit_sizes)+*
            }
        }

        impl BitsReader for #ident {
            fn read(input: &BitSlice<Msb0, u8>, bit_size: usize) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let (bits, new_rest) = input.split_at(bit_size);

                let mut rest = bits;
                #(#field_variables)*
                let value = #initialize_struct;

                Ok((new_rest, value))
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}
