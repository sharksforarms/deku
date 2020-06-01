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

    let (imp, ty, wher) = input.generics.split_for_impl();

    let ident = &input.ident;
    let ident = quote! { #ident #ty };

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("expected `struct` type");

    let field_writes = emit_field_writes(input, &fields, Some(quote! { self. }))?;
    let field_updates = emit_field_updates(&fields, Some(quote! { self. }))?;

    tokens.extend(quote! {
        impl #imp std::convert::TryFrom<#ident> for BitVec<Msb0, u8> #wher {
            type Error = DekuError;

            fn try_from(input: #ident) -> Result<Self, Self::Error> {
                input.to_bitvec()
            }
        }

        impl #imp std::convert::TryFrom<#ident> for Vec<u8> #wher {
            type Error = DekuError;

            fn try_from(input: #ident) -> Result<Self, Self::Error> {
                input.to_bytes()
            }
        }

        impl #imp #ident #wher {

            pub fn update(&mut self) -> Result<(), DekuError> {
                use std::convert::TryInto;
                #(#field_updates)*

                Ok(())
            }

            pub fn to_bytes(&self) -> Result<Vec<u8>, DekuError> {
                let mut acc: BitVec<Msb0, u8> = self.to_bitvec()?;
                Ok(acc.into_vec())
            }

            pub fn to_bitvec(&self) -> Result<BitVec<Msb0, u8>, DekuError> {
                let mut acc: BitVec<Msb0, u8> = BitVec::new();

                #(#field_writes)*

                Ok(acc)
            }
        }

        impl #imp BitsWriter for #ident #wher {
            fn write(&self, output_is_le: bool, bit_size: Option<usize>) -> Result<BitVec<Msb0, u8>, DekuError> {
                self.to_bitvec()
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_enum(input: &DekuReceiver) -> Result<TokenStream, darling::Error> {
    let mut tokens = TokenStream::new();

    let (imp, ty, wher) = input.generics.split_for_impl();

    let variants = input
        .data
        .as_ref()
        .take_enum()
        .expect("expected `enum` type");

    let ident = &input.ident;
    let ident = quote! { #ident #ty };

    let id_type = input.id_type.as_ref().expect("expected `id_type` on enum");
    let id_is_le_bytes = input.endian == EndianNess::Little;
    let id_bit_size = super::option_as_literal_token(input.id_bits);

    let mut variant_writes = vec![];
    let mut variant_updates = vec![];

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

        let variant_write = if variant_writer.is_some() {
            quote! { #variant_writer ?; }
        } else {
            let field_writes = emit_field_writes(input, &variant.fields.as_ref(), None)?;

            quote! {
                {
                    let mut variant_id: #id_type = #variant_id;
                    let bits = variant_id.write(#id_is_le_bytes, #id_bit_size)?;
                    acc.extend(bits);

                    #(#field_writes)*
                }
            }
        };

        let variant_field_updates = emit_field_updates(&variant.fields.as_ref(), None)?;

        variant_writes.push(quote! {
            Self :: #variant_match => {
                #variant_write
            }
        });

        variant_updates.push(quote! {
            Self :: #variant_match => {
                #(#variant_field_updates)*
            }
        });
    }

    tokens.extend(quote! {
        impl #imp std::convert::TryFrom<#ident> for BitVec<Msb0, u8> #wher {
            type Error = DekuError;

            fn try_from(input: #ident) -> Result<Self, Self::Error> {
                input.to_bitvec()
            }
        }

        impl #imp std::convert::TryFrom<#ident> for Vec<u8> #wher {
            type Error = DekuError;

            fn try_from(input: #ident) -> Result<Self, Self::Error> {
                input.to_bytes()
            }
        }

        impl #imp #ident #wher {
            pub fn update(&mut self) -> Result<(), DekuError> {
                use std::convert::TryInto;

                match self {
                    #(#variant_updates),*
                }

                Ok(())
            }

            pub fn to_bytes(&self) -> Result<Vec<u8>, DekuError> {
                let mut acc: BitVec<Msb0, u8> = self.to_bitvec()?;
                Ok(acc.into_vec())
            }

            pub fn to_bitvec(&self) -> Result<BitVec<Msb0, u8>, DekuError> {
                let mut acc: BitVec<Msb0, u8> = BitVec::new();

                match self {
                    #(#variant_writes),*
                }

                Ok(acc)
            }
        }

        impl #imp BitsWriter for #ident #wher {
            fn write(&self, output_is_le: bool, bit_size: Option<usize>) -> Result<BitVec<Msb0, u8>, DekuError> {
                self.to_bitvec()
            }
        }
    });

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_field_writes(
    input: &DekuReceiver,
    fields: &Fields<&DekuFieldReceiver>,
    object_prefix: Option<TokenStream>,
) -> Result<Vec<TokenStream>, darling::Error> {
    let mut field_writes = vec![];

    for (i, f) in fields.iter().enumerate() {
        let field_write = emit_field_write(input, i, f, &object_prefix)?;
        field_writes.push(field_write);
    }

    Ok(field_writes)
}

fn emit_field_updates(
    fields: &Fields<&DekuFieldReceiver>,
    object_prefix: Option<TokenStream>,
) -> Result<Vec<TokenStream>, darling::Error> {
    let mut field_updates = vec![];

    for (i, f) in fields.iter().enumerate() {
        let new_field_updates = emit_field_update(i, f, &object_prefix)?;
        field_updates.extend(new_field_updates);
    }

    Ok(field_updates)
}

fn emit_field_update(
    i: usize,
    f: &DekuFieldReceiver,
    object_prefix: &Option<TokenStream>,
) -> Result<Vec<TokenStream>, darling::Error> {
    assert!(
        f.bytes.is_none(),
        "dev error: `bytes` should be None, use `bits` to get size"
    );
    let mut field_updates = vec![];

    let field_len = f.get_len_field(i, object_prefix.is_none());
    let field_ident = f.get_ident(i, object_prefix.is_none());

    let deref = if object_prefix.is_none() {
        Some(quote! { * })
    } else {
        None
    };

    // If `len` attr is provided, overwrite the field with the .len() of the container
    if let Some(field_len) = field_len {
        field_updates.push(quote! {
            #deref #object_prefix #field_len = #object_prefix #field_ident.len().try_into()?;
        });
    }

    Ok(field_updates)
}

fn emit_field_write(
    input: &DekuReceiver,
    i: usize,
    f: &DekuFieldReceiver,
    object_prefix: &Option<TokenStream>,
) -> Result<TokenStream, darling::Error> {
    assert!(
        f.bytes.is_none(),
        "dev error: `bytes` should be None, use `bits` to get size"
    );

    let is_le_bytes = f.endian.unwrap_or(input.endian) == EndianNess::Little;
    let field_bits = super::option_as_literal_token(f.bits);
    let field_writer = &f.writer;
    let field_ident = f.get_ident(i, object_prefix.is_none());

    let field_write_func = if field_writer.is_some() {
        quote! { #field_writer }
    } else {
        quote! { #object_prefix #field_ident.write(output_is_le, field_bits) }
    };

    let field_write = quote! {
        let output_is_le = #is_le_bytes;
        let field_bits = #field_bits;

        let bits = #field_write_func ?;
        acc.extend(bits);
    };

    Ok(field_write)
}
