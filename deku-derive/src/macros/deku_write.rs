use crate::{DekuFieldReceiver, DekuReceiver, EndianNess};
use darling::ast::{Data, Fields};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_write(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
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

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type");

    let (field_overwrites, field_writes) =
        emit_field_writes(input, &fields, Some(quote! { input. }))?;

    tokens.extend(quote! {
        impl<P> From<#ident> for BitVec<P, u8> where P: BitOrder {
            fn from(mut input: #ident) -> Self {
                use std::convert::TryInto;

                let mut acc: BitVec<P, u8> = BitVec::new();

                #(#field_overwrites)*

                #(#field_writes)*

                acc
            }
        }

        impl From<#ident> for Vec<u8> {
            fn from(mut input: #ident) -> Self {
                let mut acc: BitVec<Msb0, u8> = input.into();
                acc.into_vec()
            }
        }

        impl BitsWriter for #ident {
            fn write(self, output_is_le: bool, bit_size: Option<usize>) -> BitVec<Msb0, u8> {
                self.into()
            }
        }
    });

    println!("{}", tokens.to_string());
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
        let variant_writer = &variant.reader;

        let field_idents = variant
            .fields
            .as_ref()
            .iter()
            .enumerate()
            .map(|(i, f)| f.get_ident(i, true))
            .collect::<Vec<_>>();
        let variant_match = super::gen_enum_init(variant_is_named, variant_ident, field_idents);

        let variant_write_func = if variant_writer.is_some() {
            quote! { #variant_writer; }
        } else {
            let (field_overwrites, field_writes) =
                emit_field_writes(input, &variant.fields.as_ref(), None)?;

            quote! {
                {
                    let bits = (#variant_id as #id_type).write(#id_is_le_bytes, #id_bit_size);
                    acc.extend(bits);

                    #(#field_overwrites)*
                    #(#field_writes)*
                }
            }
        };

        variant_matches.push(quote! {
            #ident :: #variant_match => {
                #variant_write_func
            }
        });
    }

    tokens.extend(quote! {
        impl<P> From<#ident> for BitVec<P, u8> where P: BitOrder {
            fn from(input: #ident) -> Self {
                use std::convert::TryInto;

                let mut acc: BitVec<P, u8> = BitVec::new();

                match input {
                    #(#variant_matches),*
                }

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
            fn write(self, output_is_le: bool, bit_size: Option<usize>) -> BitVec<Msb0, u8> {
                self.into()
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_field_writes(
    input: &DekuReceiver,
    fields: &Fields<&DekuFieldReceiver>,
    field_accessor: Option<TokenStream>,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>), darling::Error> {
    let mut field_overwrites = vec![];
    let mut field_writes = vec![];

    for (i, f) in fields.iter().enumerate() {
        let (_field_idents, new_field_overwrites, field_write) =
            emit_field_write(input, i, f, &field_accessor)?;

        field_overwrites.extend(new_field_overwrites);
        field_writes.push(field_write);
    }

    Ok((field_overwrites, field_writes))
}

fn emit_field_write(
    input: &DekuReceiver,
    i: usize,
    f: &DekuFieldReceiver,
    field_accessor: &Option<TokenStream>,
) -> Result<(TokenStream, Vec<TokenStream>, TokenStream), darling::Error> {
    assert!(
        f.bytes.is_none(),
        "dev error: `bytes` should be None, use `bits` to get size"
    );

    let mut field_overwrites = vec![];

    let is_le_bytes = f.endian.unwrap_or(input.endian) == EndianNess::Little;
    let field_bits = super::option_as_literal_token(f.bits);
    let field_writer = &f.writer;
    let field_len_prefix = f.get_len_field(i, true);
    let field_len = f.get_len_field(i, false);
    let field_ident = f.get_ident(i, field_accessor.is_none());

    let mut_ref = if field_accessor.is_some() {
        Some(quote! { &mut })
    } else {
        None
    };

    let deref = if field_accessor.is_some() {
        Some(quote! { * })
    } else {
        None
    };

    let is_mut = if field_accessor.is_none() {
        Some(quote! { mut })
    } else {
        None
    };

    // If `len` attr is provided, overwrite the field with the .len() of the container
    if let Some(field_len) = field_len {
        field_overwrites.push(quote! {
            // first, copy the field to get it's type
            let #is_mut #field_len_prefix = #mut_ref #field_accessor #field_len;
            // then modify it. Otherwise, we'd need the type of the `len` field
            #deref #field_len_prefix = #field_accessor #field_ident.len().try_into().unwrap(); // TODO: unwrap
        });
    }

    let field_write_func = if field_writer.is_some() {
        quote! { #field_writer }
    } else {
        quote! { field_val.write(output_is_le, field_bits) }
    };

    let field_write = quote! {
        let field_val = #field_accessor #field_ident;
        let output_is_le = #is_le_bytes;
        let field_bits = #field_bits;

        let bits = #field_write_func;
        acc.extend(bits);
    };

    Ok((field_ident, field_overwrites, field_write))
}
