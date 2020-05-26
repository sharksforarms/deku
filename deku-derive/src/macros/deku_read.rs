use crate::{DekuFieldReceiver, DekuReceiver, EndianNess};
use darling::ast::{Data, Fields};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_read(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    assert!(
        input.id_bytes.is_none(),
        "dev error: `id_bytes` should be None, use `id_bits` to get size"
    );

    match &input.data {
        Data::Enum(_) => emit_enum(input),
        Data::Struct(_) => emit_struct(input),
    }
}

fn emit_struct(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    let mut tokens = TokenStream::new();

    let ident = &input.ident;

    let fields = &input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type");

    // check if the first field has an ident, if not, it's a unnamed struct
    let is_named_struct = fields
        .fields
        .get(0)
        .and_then(|v| v.ident.as_ref())
        .is_some();

    let (field_idents, field_reads) = emit_field_reads(input, &fields)?;

    let initialize_struct = super::gen_struct_init(is_named_struct, field_idents);

    tokens.extend(quote! {
        impl std::convert::TryFrom<&[u8]> for #ident {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
                let (rest, res) = Self::from_bytes(input)?;
                Ok(res)
            }
        }

        impl #ident {
            fn from_bytes(input: &[u8]) -> Result<(&[u8], Self), DekuError> {
                let mut rest = input.bits::<Msb0>();

                #(#field_reads)*
                let value = #initialize_struct;

                Ok((rest.as_slice(), value))
            }
        }

        impl BitsReader for #ident {
            fn read(input: &BitSlice<Msb0, u8>, _input_is_le: bool, _bit_size: Option<usize>) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {

                let mut rest = input;
                #(#field_reads)*
                let value = #initialize_struct;

                Ok((rest, value))
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_enum(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    let mut tokens = TokenStream::new();

    let variants = input
        .data
        .as_ref()
        .take_enum()
        .expect("expected `enum` type");

    let ident = &input.ident;
    let id_type = input.id_type.as_ref().expect("expected `id_type` on enum");
    let id_is_le_bytes = input.endian == EndianNess::Little;
    let id_bit_size = super::option_as_literal_token(input.id_bits);

    let variant_id_read = {
        quote! {
            {
                let (new_rest, variant_id) = #id_type :: read (rest, #id_is_le_bytes, #id_bit_size)?;
                rest = new_rest;

                variant_id
            }
        }
    };

    let mut variant_matches = vec![];

    for (_i, variant) in variants.into_iter().enumerate() {
        // check if the first field has an ident, if not, it's a unnamed struct
        let variant_is_named = variant
            .fields
            .fields
            .get(0)
            .and_then(|v| v.ident.as_ref())
            .is_some();

        let variant_id: TokenStream = variant.id.parse().unwrap();
        let variant_ident = &variant.ident;
        let variant_reader = &variant.reader;

        let variant_read_func = if variant_reader.is_some() {
            quote! { #variant_reader; }
        } else {
            let (field_idents, field_reads) = emit_field_reads(input, &variant.fields.as_ref())?;

            let initialize_enum =
                super::gen_enum_init(variant_is_named, variant_ident, field_idents);

            quote! {
                {
                    #(#field_reads)*
                    Self :: #initialize_enum
                }
            }
        };

        variant_matches.push(quote! {
            #variant_id => {
                #variant_read_func
            }
        });
    }

    tokens.extend(quote! {
        impl std::convert::TryFrom<&[u8]> for #ident {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
                let (rest, res) = Self::from_bytes(input)?;
                Ok(res)
            }
        }

        impl #ident {
            fn from_bytes(input: &[u8]) -> Result<(&[u8], Self), DekuError> {
                let mut rest = input.bits::<Msb0>();

                let variant_id = #variant_id_read;

                let value = match variant_id {
                    #(#variant_matches),*

                    _ => {
                        return Err(DekuError::Parse(format!("Could not match enum variant id = {:?}", variant_id)));
                    }
                };

                Ok((rest.as_slice(), value))
            }
        }

        impl BitsReader for #ident {
            fn read(input: &BitSlice<Msb0, u8>, _input_is_le: bool, _bit_size: Option<usize>) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                let mut rest = input;

                let variant_id = #variant_id_read;

                let value = match variant_id {
                    #(#variant_matches),*

                    _ => {
                        return Err(DekuError::Parse(format!("Could not find enum variant id = {:?}", variant_id)));
                    }
                };

                Ok((rest, value))
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_field_reads(
    input: &DekuReceiver,
    fields: &Fields<&DekuFieldReceiver>,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>), darling::Error> {
    let mut field_reads = vec![];
    let mut field_idents = vec![];

    for (i, f) in fields.iter().enumerate() {
        let (field_ident, field_read) = emit_field_read(input, i, f)?;
        field_idents.push(field_ident);
        field_reads.push(field_read);
    }

    Ok((field_idents, field_reads))
}

fn emit_field_read(
    input: &DekuReceiver,
    i: usize,
    f: &DekuFieldReceiver,
) -> Result<(TokenStream, TokenStream), darling::Error> {
    assert!(
        f.bytes.is_none(),
        "dev error: `bytes` should be None, use `bits` to get size"
    );

    let field_type = super::wrap_turbofish(&f.ty);
    let is_le_bytes = f.endian.unwrap_or(input.endian) == EndianNess::Little;
    let field_bits = super::option_as_literal_token(f.bits);
    let field_reader = &f.reader;
    let field_len = f.get_len_field(i, true);
    let field_ident = f.get_ident(i, true);

    let field_read_func = if field_reader.is_some() {
        quote! { #field_reader }
    } else if field_len.is_some() {
        quote! { #field_type::read(rest, input_is_le, field_bits, #field_len as usize) }
    } else {
        quote! { #field_type::read(rest, input_is_le, field_bits) }
    };

    let field_read = quote! {
        let #field_ident = {
            let field_bits = #field_bits;
            let input_is_le = #is_le_bytes;

            let read_ret = #field_read_func;
            let (new_rest, value) = read_ret?;

            rest = new_rest;

            value
        };
    };

    Ok((field_ident, field_read))
}
