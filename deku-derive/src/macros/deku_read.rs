use crate::macros::{gen_ctx_types_and_arg, gen_hidden_field_ident, gen_hidden_field_idents};
use crate::{DekuData, EndianNess, FieldData};
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

    let hidden_fields = gen_hidden_field_idents(is_named_struct, field_idents);

    let initialize_struct = super::gen_struct_init(is_named_struct, hidden_fields);

    // Only implement `DekuContainerRead` for types don't need any context.
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

            let hidden_fields = gen_hidden_field_idents(variant_is_named, field_idents);
            let initialize_enum =
                super::gen_enum_init(variant_is_named, variant_ident, hidden_fields);

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

    // Only implement `DekuContainerRead` for types don't need any context.
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
    _input: &DekuData,
    i: usize,
    f: &FieldData,
) -> Result<(TokenStream, TokenStream), syn::Error> {
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
    let hidden_field_ident = gen_hidden_field_ident(field_ident.clone());

    let field_read_func = if field_reader.is_some() {
        quote! { #field_reader }
    } else {
        let mut read_args = Vec::with_capacity(3);
        if let Some(field_is_le) = field_is_le {
            read_args.push(quote! {#field_is_le});
        }
        if let Some(field_bits) = f.bits {
            read_args.push(quote! {#field_bits})
        }
        if let Some(ctx) = &f.ctx {
            read_args.push(quote! {#ctx});
        }

        // Because `impl DekuRead<(bool, usize)>` but `impl DekuRead<bool>`(not tuple)
        let read_args = if read_args.len() == 1 {
            let arg = &read_args[0];
            quote! {#arg}
        } else {
            quote! {#(#read_args),*}
        };

        // Count is special, we need to generate `(count, (other, ..))` for it.
        if let Some(field_count) = &f.count {
            // The count has same problem, when it isn't a copy type, the field will be moved.
            // e.g. struct FooBar {
            //   a: Baz // a type implement `Into<usize>` but not `Copy`.
            //   #[deku(count = "a") <-- Oops, use of moved value: `a`
            //   b: Vec<_>
            // }
            quote! {DekuRead::read(rest, (usize::try_from(#field_count << 0usize)?, (#read_args)))}
        } else {
            quote! {DekuRead::read(rest, (#read_args))}
        }
    };

    // We must pass a ref to context so that it won't be moved when use.
    // e.g.
    // let a = read(rest);
    // let b = read(rest, a); <-- Oops! a have been moved, then we can't use it for constructing.
    // let c = read(rest, &mut b); <-- `b` will be changed.
    // So I add a `__`(double underscore) for it. Hopes none writes `let d = read(rest, __b);`
    let field_read = quote! {
        let #hidden_field_ident = {
            let (new_rest, value) = #field_read_func?;
            let value: #field_type = #field_map(value)?;

            rest = new_rest;

            value
        };
        let #field_ident = &#hidden_field_ident;
    };

    if f.skip {
        let default_tok = f.default.as_ref().expect("expected `default` attribute");

        let default_read = quote! {
            let #hidden_field_ident = {
                #default_tok
            };
            let #field_ident = &#hidden_field_ident;
        };
        return Ok((field_ident, default_read));
    }

    Ok((field_ident, field_read))
}
