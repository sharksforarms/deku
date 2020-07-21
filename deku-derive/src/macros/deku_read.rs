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

    let (imp, ty, wher) = input.generics.split_for_impl();

    let ident = &input.ident;
    let ident = quote! { #ident #ty };

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
        impl #imp core::convert::TryFrom<&[u8]> for #ident #wher {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
                let (rest, res) = Self::from_bytes((input, 0))?;
                if !rest.0.is_empty() {
                    return Err(DekuError::Parse(format!("Too much data")));
                }
                Ok(res)
            }
        }

        impl #imp DekuContainerRead for #ident #wher {
            fn from_bytes(input: (&[u8], usize)) -> Result<((&[u8], usize), Self), DekuError> {
                use core::convert::TryFrom;
                let input_bits = input.0.bits::<Msb0>();

                let mut rest = input.0.bits::<Msb0>();
                rest = &rest[input.1..];

                #(#field_reads)*
                let value = #initialize_struct;

                let pad = 8 * ((rest.len() + 7) / 8) - rest.len();
                let read_idx = input_bits.len() - (rest.len() + pad);

                Ok(((&input_bits[read_idx..].as_slice(), pad), value))
            }
        }

        impl #imp DekuRead for #ident #wher {
            fn read(input: &BitSlice<Msb0, u8>, _: ()) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                use core::convert::TryFrom;
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

    let (imp, ty, wher) = input.generics.split_for_impl();

    let variants = input
        .data
        .as_ref()
        .take_enum()
        .expect("expected `enum` type");

    let ident = &input.ident;
    let ident = quote! { #ident #ty };

    // TODO: replace `expect` with an error.
    let id_type = input.id_type.as_ref().expect("expected `id_type` on enum");

    let id_is_le_bytes = input.endian == EndianNess::Little;

    let id_args = if let Some(id_bit_size) = input.id_bits {
        quote! {(#id_is_le_bytes, #id_bit_size)}
    } else {
        quote! {#id_is_le_bytes}
    };

    let mut variant_matches = vec![];
    let mut has_default_match = false;

    for (_i, variant) in variants.into_iter().enumerate() {
        // check if the first field has an ident, if not, it's a unnamed struct
        let variant_is_named = variant
            .fields
            .fields
            .get(0)
            .and_then(|v| v.ident.as_ref())
            .is_some();

        let variant_id = if let Some(variant_id) = &variant.id {
            variant_id.parse().unwrap()
        } else {
            // id attribute not provided, treat it as a catch-all default
            has_default_match = true;
            quote! { _ }
        };

        let variant_ident = &variant.ident;
        let variant_reader = &variant.reader;

        let variant_read_func = if variant_reader.is_some() {
            quote! { #variant_reader; }
        } else {
            let (field_idents, field_reads) = emit_field_reads(input, &variant.fields.as_ref())?;

            let initialize_enum =
                super::gen_enum_init(variant_is_named, variant_ident, field_idents);

            // if we're consuming an id, set the rest to new_rest before reading the variant
            let new_rest = if variant.id.is_some() {
                quote! {
                    rest = new_rest;
                }
            } else {
                quote! {}
            };

            quote! {
                {
                    #new_rest
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

    // if no default match, return error
    if !has_default_match {
        variant_matches.push(quote! {
            _ => {
                return Err(DekuError::Parse(format!("Could not match enum variant id = {:?}", variant_id)));
            }
        });
    }

    let variant_read = quote! {
        let (new_rest, variant_id) = #id_type::read(rest, #id_args)?;

        let value = match variant_id {
            #(#variant_matches),*
        };
    };

    tokens.extend(quote! {
        impl #imp core::convert::TryFrom<&[u8]> for #ident #wher {
            type Error = DekuError;

            fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
                let (rest, res) = Self::from_bytes((input, 0))?;
                if !rest.0.is_empty() {
                    return Err(DekuError::Parse(format!("Too much data")));
                }
                Ok(res)
            }
        }

        impl #imp DekuContainerRead for #ident #wher {
            fn from_bytes(input: (&[u8], usize)) -> Result<((&[u8], usize), Self), DekuError> {
                use core::convert::TryFrom;
                let input_bits = input.0.bits::<Msb0>();

                let mut rest = input.0.bits::<Msb0>();
                rest = &rest[input.1..];

                #variant_read

                let pad = 8 * ((rest.len() + 7) / 8) - rest.len();
                let read_idx = input_bits.len() - (rest.len() + pad);

                Ok(((&input_bits[read_idx..].as_slice(), pad), value))
            }
        }

        impl #imp DekuRead for #ident #wher {
            fn read(input: &BitSlice<Msb0, u8>, _: ()) -> Result<(&BitSlice<Msb0, u8>, Self), DekuError> {
                use core::convert::TryFrom;
                let mut rest = input;

                #variant_read

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
    _input: &DekuReceiver,
    i: usize,
    f: &DekuFieldReceiver,
) -> Result<(TokenStream, TokenStream), darling::Error> {
    assert!(
        f.bytes.is_none(),
        "dev error: `bytes` should be None, use `bits` to get size"
    );

    let field_type = &f.ty;
    let field_is_le = f.endian.map(|endian| endian == EndianNess::Little);
    let field_reader = &f.reader;
    let field_map = f
        .map
        .as_ref()
        .map(|v| {
            quote! { (#v) }
        })
        .or_else(|| Some(quote! { Result::<_, DekuError>::Ok }));
    let field_ident = f.get_ident(i, true);

    let field_read_func = if field_reader.is_some() {
        quote! { #field_reader }
    } else {
        let mut read_args = Vec::with_capacity(3);

        if let Some(field_count) = &f.count {
            read_args.push(quote! {usize::try_from(#field_count)?})
        }
        if let Some(field_is_le) = field_is_le {
            read_args.push(quote! {#field_is_le});
        }
        if let Some(field_bits) = f.bits {
            read_args.push(quote! {#field_bits})
        }

        // Because `impl DekuRead<(bool, usize)>` but `impl DekuRead<bool>`(not tuple)
        let read_args = if read_args.len() == 1 {
            let arg = &read_args[0];
            quote! {#arg}
        } else {
            quote! {#(#read_args),*}
        };

        quote! {DekuRead::read(rest, (#read_args))}
    };

    let field_read = quote! {
        let #field_ident = {
            let (new_rest, value) = #field_read_func?;
            let value: #field_type = #field_map(value)?;

            rest = new_rest;

            value
        };
    };

    if f.skip {
        let default_tok = f.default.as_ref().expect("expected `default` attribute");

        let default_read = quote! {
            let #field_ident = {
                #default_tok
            };
        };
        return Ok((field_ident, default_read));
    }

    Ok((field_ident, field_read))
}
