use crate::{DekuReceiver, EndianNess};
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

    // Iterate each field, creating tokens for implementations
    for (i, f) in fields.into_iter().enumerate() {
        let field_type = {
            let t = &f.ty;
            quote! { <#t> }
        };

        let field_endian = f.endian.unwrap_or(input.endian);
        let field_bits = f.bits;
        let field_bytes = f.bytes;
        let field_reader = f.reader.as_ref().map(|fn_str| {
            let fn_ident: TokenStream = fn_str.parse().unwrap();

            // TODO: Assert the shape of fn_ident? Only allow a structured function call instead of anything?
            quote! { #fn_ident; }
        });

        let field_len = &f.len.as_ref().map(|v| {
            let field_name = if f.ident.is_some() {
                // Named
                v.to_string()
            } else {
                // Unnamed
                format!("field_{}", v)
            };

            syn::Ident::new(&field_name, syn::export::Span::call_site())
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
        let field_bits = match field_bits.or_else(|| field_bytes.map(|v| v * 8usize)) {
            Some(b) => quote! {Some(#b)},
            None => quote! {None},
        };

        let is_le_bytes = field_endian == EndianNess::Little;

        let field_read_func = if field_reader.is_some() {
            quote! { #field_reader }
        } else if field_len.is_some() {
            quote! { #field_type::read(rest, #is_le_bytes, field_bits, #field_len as usize) }
        } else {
            quote! { #field_type::read(rest, #is_le_bytes, field_bits) }
        };

        // Create field read token for TryFrom trait
        let field_read = quote! {
            let #field_ident = {
                let field_bits = #field_bits;

                let read_ret = #field_read_func;
                let (new_rest, value) = read_ret?;

                rest = new_rest;

                value
            };
        };

        field_variables.push(field_read);
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

                let (rest, res) = Self::from_bytes(input)?;

                Ok(res)
            }
        }

        impl #ident {
            fn from_bytes(input: &[u8]) -> Result<(&[u8], Self), DekuError> {
                let mut rest = input.bits::<Msb0>();

                #(#field_variables)*
                let value = #initialize_struct;

                Ok((rest.as_slice(), value))
            }
        }

        impl BitsReader for #ident {
            fn read(input: &BitSlice<Msb0, u8>, _input_is_le: bool, _bit_size: Option<usize>) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {

                let mut rest = input;
                #(#field_variables)*
                let value = #initialize_struct;

                Ok((rest, value))
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}
