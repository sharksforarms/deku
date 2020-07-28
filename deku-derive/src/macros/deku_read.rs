use crate::macros::{
    gen_ctx_types_and_arg, gen_field_args, gen_id_args, gen_internal_field_ident,
    gen_internal_field_idents,
};
use crate::{DekuData, FieldData};
use darling::ast::{Data, Fields};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn emit_deku_read(input: &DekuData) -> Result<TokenStream, syn::Error> {
    match &input.data {
        Data::Enum(_) => emit_enum(input),
        Data::Struct(_) => emit_struct(input),
    }
}

fn emit_struct(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let mut tokens = TokenStream::new();

    let (imp, ty, wher) = input.generics.split_for_impl();

    let ident = &input.ident;
    let ident = quote! { #ident #ty };

    // checked in `emit_deku_read`
    let fields = &input.data.as_ref().take_struct().unwrap();

    // check if the first field has an ident, if not, it's a unnamed struct
    let is_named_struct = fields
        .fields
        .get(0)
        .and_then(|v| v.ident.as_ref())
        .is_some();

    let (field_idents, field_reads) = emit_field_reads(input, &fields)?;

    let internal_fields = gen_internal_field_idents(is_named_struct, field_idents);

    let initialize_struct = super::gen_struct_init(is_named_struct, internal_fields);

    // Implement `DekuContainerRead` for types that don't need a context
    if input.ctx.is_none() {
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
        }})
    }

    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    tokens.extend(quote! {
        impl #imp DekuRead<#ctx_types> for #ident #wher {
            fn read<'a>(input: &'a BitSlice<Msb0, u8>, #ctx_arg) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError> {
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

fn emit_enum(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let mut tokens = TokenStream::new();

    let (imp, ty, wher) = input.generics.split_for_impl();

    // checked in `emit_deku_read`
    let variants = input.data.as_ref().take_enum().unwrap();

    let ident = &input.ident;
    let ident = quote! { #ident #ty };

    // checked in `DekuData::validate`
    let id_type = input.id_type.as_ref().unwrap();

    let id_args = gen_id_args(input.endian.as_ref(), input.id_bits)?;

    let mut variant_matches = vec![];
    let mut has_default_match = false;

    /*
    FIXME: The loop body is too big
    */
    for variant in variants {
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

            let internal_fields = gen_internal_field_idents(variant_is_named, field_idents);
            let initialize_enum =
                super::gen_enum_init(variant_is_named, variant_ident, internal_fields);

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
        let (new_rest, variant_id) = #id_type::read(rest, (#id_args))?;

        let value = match variant_id {
            #(#variant_matches),*
        };
    };

    // Implement `DekuContainerRead` for types that don't need a context
    if input.ctx.is_none() {
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
        })
    }

    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    tokens.extend(quote! {
        impl #imp DekuRead<#ctx_types> for #ident #wher {
            fn read<'a>(input: &'a BitSlice<Msb0, u8>, #ctx_arg) -> Result<(&'a BitSlice<Msb0, u8>, Self), DekuError> {
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
    input: &DekuData,
    fields: &Fields<&FieldData>,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>), syn::Error> {
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
    input: &DekuData,
    i: usize,
    f: &FieldData,
) -> Result<(TokenStream, TokenStream), syn::Error> {
    let field_type = &f.ty;

    let field_endian = f.endian.as_ref().or_else(|| input.endian.as_ref());

    let field_reader = &f.reader;

    let field_map = f
        .map
        .as_ref()
        .map(|v| {
            quote! { (#v) }
        })
        .or_else(|| Some(quote! { Result::<_, DekuError>::Ok }));

    let field_ident = f.get_ident(i, true);

    let internal_field_ident = gen_internal_field_ident(field_ident.clone());

    let field_read_func = if field_reader.is_some() {
        quote! { #field_reader }
    } else {
        let read_args = gen_field_args(field_endian, f.bits, f.ctx.as_ref())?;

        // Count is special, we need to generate `(count, (other, ..))` for it.
        if let Some(field_count) = &f.count {
            // The count has same problem, when it isn't a copy type, the field will be moved.
            // e.g. struct FooBar {
            //   a: Baz // a type implement `Into<usize>` but not `Copy`.
            //   #[deku(count = "a") <-- Oops, use of moved value: `a`
            //   b: Vec<_>
            // }
            quote! {
                {
                    use core::borrow::Borrow;
                    DekuRead::read(rest, (deku::ctx::Count(usize::try_from(*((#field_count).borrow()))?), (#read_args)))
                }
            }
        } else {
            quote! {DekuRead::read(rest, (#read_args))}
        }
    };

    let field_read_normal = quote! {
        let (new_rest, value) = #field_read_func?;
        let value: #field_type = #field_map(value)?;

        rest = new_rest;

        value
    };
    let field_default = &f.default;

    let field_read_tokens = match (f.skip, &f.cond) {
        (true, Some(field_cond)) => {
            // #[deku(skip, cond = "...")] ==> `skip` if `cond`
            quote! {
                if (#field_cond) {
                    #field_default
                } else {
                    #field_read_normal
                }
            }
        }
        (true, None) => {
            // #[deku(skip)] ==> `skip`
            quote! {
                #field_default
            }
        }
        (false, Some(field_cond)) => {
            // #[deku(cond = "...")] ==> read if `cond`
            quote! {
                if (#field_cond) {
                    #field_read_normal
                } else {
                    #field_default
                }
            }
        }
        (false, None) => {
            quote! {
                #field_read_normal
            }
        }
    };

    let field_read = quote! {
        let #internal_field_ident = {
            #field_read_tokens
        };
        let #field_ident = &#internal_field_ident;
    };

    Ok((field_ident, field_read))
}
