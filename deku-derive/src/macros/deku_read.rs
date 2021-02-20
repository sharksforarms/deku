use crate::{
    macros::{
        gen_ctx_types_and_arg, gen_field_args, gen_id_args, gen_internal_field_ident,
        gen_internal_field_idents, pad_bits, gen_type_from_ctx_id, token_contains_string, wrap_default_ctx,
    },
    Id,
};
use crate::{DekuData, FieldData};
use darling::{
    ast::{Data, Fields},
    ToTokens,
};
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

    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map_or(quote!('_), |v| quote!(#v));

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

    let magic_read = emit_magic_read(input);

    let (field_idents, field_reads) = emit_field_reads(input, &fields)?;

    // filter out temporary fields
    let field_idents: Vec<&TokenStream> = field_idents
        .iter()
        .filter(|f| !f.is_temp)
        .map(|f| &f.field_ident)
        .collect();

    let internal_fields = gen_internal_field_idents(is_named_struct, field_idents);

    let initialize_struct = super::gen_struct_init(is_named_struct, internal_fields);

    // Implement `DekuContainerRead` for types that don't need a context
    if input.ctx.is_none() || (input.ctx.is_some() && input.ctx_default.is_some()) {
        let from_bytes_body = wrap_default_ctx(
            quote! {
                use core::convert::TryFrom;
                let __deku_input_bits = __deku_input.0.view_bits::<Msb0>();

                let mut __deku_rest = __deku_input_bits;
                __deku_rest = &__deku_rest[__deku_input.1..];

                #magic_read

                #(#field_reads)*
                let __deku_value = #initialize_struct;

                let __deku_pad = 8 * ((__deku_rest.len() + 7) / 8) - __deku_rest.len();
                let __deku_read_idx = __deku_input_bits.len() - (__deku_rest.len() + __deku_pad);

                Ok(((__deku_input_bits[__deku_read_idx..].as_raw_slice(), __deku_pad), __deku_value))
            },
            &input.ctx,
            &input.ctx_default,
        );

        tokens.extend(emit_try_from(&imp, &lifetime, &ident, wher));

        tokens.extend(emit_from_bytes(
            &imp,
            &lifetime,
            &ident,
            wher,
            from_bytes_body,
        ));
    }

    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    let read_body = quote! {
        use core::convert::TryFrom;
        let mut __deku_rest = __deku_input_bits;

        #magic_read

        #(#field_reads)*
        let __deku_value = #initialize_struct;

        Ok((__deku_rest, __deku_value))
    };

    tokens.extend(quote! {
        impl #imp DekuRead<#lifetime, #ctx_types> for #ident #wher {
            fn read(__deku_input_bits: &#lifetime BitSlice<Msb0, u8>, #ctx_arg) -> Result<(&#lifetime BitSlice<Msb0, u8>, Self), DekuError> {
                #read_body
            }
        }
    });

    if input.ctx.is_some() && input.ctx_default.is_some() {
        let read_body = wrap_default_ctx(read_body, &input.ctx, &input.ctx_default);

        tokens.extend(quote! {
            impl #imp DekuRead<#lifetime> for #ident #wher {
                fn read(__deku_input_bits: &#lifetime BitSlice<Msb0, u8>, _: ()) -> Result<(&#lifetime BitSlice<Msb0, u8>, Self), DekuError> {
                    #read_body
                }
            }
        });
    }

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_enum(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let mut tokens = TokenStream::new();

    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map_or(quote!('_), |v| quote!(#v));

    let (imp, ty, wher) = input.generics.split_for_impl();

    // checked in `emit_deku_read`
    let variants = input.data.as_ref().take_enum().unwrap();

    let ident = &input.ident;
    let ident = quote! { #ident #ty };
    let ident_as_string = ident.to_string();

    let id = input.id.as_ref();
    let id_type = input.id_type.as_ref();

    let id_args = gen_id_args(
        input.endian.as_ref(),
        input.bits.as_ref(),
        input.bytes.as_ref(),
    )?;

    let magic_read = emit_magic_read(input);

    let mut has_default_match = false;
    let mut pre_match_tokens = vec![];
    let mut variant_matches = vec![];
    let mut deku_ids = vec![];

    let has_discriminant = variants.iter().any(|v| v.discriminant.is_some());

    for variant in variants {
        // check if the first field has an ident, if not, it's a unnamed struct
        let variant_is_named = variant
            .fields
            .fields
            .get(0)
            .and_then(|v| v.ident.as_ref())
            .is_some();

        let (consume_id, variant_id) = if let Some(variant_id) = &variant.id {
            match variant_id {
                Id::TokenStream(v) => (true, quote! {&#v}.into_token_stream()),
                Id::LitByteStr(v) => (true, v.into_token_stream()),
            }
        } else if let Some(variant_id_pat) = &variant.id_pat {
            (false, variant_id_pat.clone())
        } else if has_discriminant {
            let ident = &variant.ident;
            let internal_ident = gen_internal_field_ident(&quote!(#ident));
            pre_match_tokens.push(quote! {
                let #internal_ident = <#id_type>::try_from(Self::#ident as isize)?;
            });
            (true, quote! { _ if __deku_variant_id == #internal_ident })
        } else {
            return Err(syn::Error::new(
                variant.ident.span(),
                "DekuRead: `id` must be specified on non-unit variants",
            ));
        };

        if variant_id.to_string() == "_" {
            has_default_match = true;
        }

        let variant_ident = &variant.ident;
        let variant_reader = &variant.reader;

        let variant_read_func = if variant_reader.is_some() {
            quote! { #variant_reader; }
        } else {
            let (field_idents, field_reads) = emit_field_reads(input, &variant.fields.as_ref())?;

            // filter out temporary fields
            let field_idents: Vec<&TokenStream> = field_idents
                .iter()
                .filter(|f| !f.is_temp)
                .map(|f| &f.field_ident)
                .collect();

            let internal_fields = gen_internal_field_idents(variant_is_named, field_idents);
            let initialize_enum =
                super::gen_enum_init(variant_is_named, variant_ident, internal_fields);

            if let Some(variant_id) = &variant.id {
                let deref = match variant_id {
                    Id::TokenStream(_) => quote! {},
                    Id::LitByteStr(_) => quote! {*},
                };

                let deku_id = quote! { Self :: #initialize_enum => Ok(#deref #variant_id)};
                deku_ids.push(deku_id);
            }

            // if we're consuming an id, set the rest to new_rest before reading the variant
            let new_rest = if consume_id {
                quote! {
                    __deku_rest = __deku_new_rest;
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
                return Err(DekuError::Parse(
                            format!(
                                "Could not match enum variant id = {:?} on enum `{}`",
                                __deku_variant_id,
                                #ident_as_string
                            )
                        ));
            }
        });
    }

    let variant_id_read = if id.is_some() {
        quote! {
            let (__deku_new_rest, __deku_variant_id) = (__deku_rest, #id);
        }
    } else if id_type.is_some() {
        quote! {
            let (__deku_new_rest, __deku_variant_id) = <#id_type>::read(__deku_rest, (#id_args))?;
        }
    } else {
        // either `id` or `type` needs to be specified
        unreachable!();
    };

    let variant_read = quote! {
        #variant_id_read

        #(#pre_match_tokens)*

        let __deku_value = match &__deku_variant_id {
            #(#variant_matches),*
        };
    };

    // Implement `DekuContainerRead` for types that don't need a context
    if input.ctx.is_none() || (input.ctx.is_some() && input.ctx_default.is_some()) {
        let from_bytes_body = wrap_default_ctx(
            quote! {
                use core::convert::TryFrom;
                let __deku_input_bits = __deku_input.0.view_bits::<Msb0>();

                let mut __deku_rest = __deku_input_bits;
                __deku_rest = &__deku_rest[__deku_input.1..];

                #magic_read

                #variant_read

                let __deku_pad = 8 * ((__deku_rest.len() + 7) / 8) - __deku_rest.len();
                let __deku_read_idx = __deku_input_bits.len() - (__deku_rest.len() + __deku_pad);

                Ok(((__deku_input_bits[__deku_read_idx..].as_raw_slice(), __deku_pad), __deku_value))
            },
            &input.ctx,
            &input.ctx_default,
        );

        tokens.extend(emit_try_from(&imp, &lifetime, &ident, wher));

        tokens.extend(emit_from_bytes(
            &imp,
            &lifetime,
            &ident,
            wher,
            from_bytes_body,
        ));
    }
    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    let read_body = quote! {
        use core::convert::TryFrom;
        let mut __deku_rest = __deku_input_bits;

        #magic_read

        #variant_read

        Ok((__deku_rest, __deku_value))
    };

    tokens.extend(quote! {
        #[allow(non_snake_case)]
        impl #imp DekuRead<#lifetime, #ctx_types> for #ident #wher {
            fn read(__deku_input_bits: &#lifetime BitSlice<Msb0, u8>, #ctx_arg) -> Result<(&#lifetime BitSlice<Msb0, u8>, Self), DekuError> {
                #read_body
            }
        }
    });

    if input.ctx.is_some() && input.ctx_default.is_some() {
        let read_body = wrap_default_ctx(read_body, &input.ctx, &input.ctx_default);

        tokens.extend(quote! {
            #[allow(non_snake_case)]
            impl #imp DekuRead<#lifetime> for #ident #wher {
                fn read(__deku_input_bits: &#lifetime BitSlice<Msb0, u8>, _: ()) -> Result<(&#lifetime BitSlice<Msb0, u8>, Self), DekuError> {
                    #read_body
                }
            }
        });
    }

    // Implement `DekuEnumExt`
    let deku_id_id_type = if let Some(id_type) = id_type {
        quote! {#id_type}
    } else {
        let r = gen_type_from_ctx_id(input.ctx.as_ref(), input.id.as_ref())?;
        quote! {#r}
    };
    let deku_id = quote! {
        impl #imp DekuEnumExt<#lifetime, #deku_id_id_type> for #ident #wher {
            fn deku_id(&self) -> Result<#deku_id_id_type, DekuError> {
                match self {
                    #(#deku_ids ,)*
                    _ => Err(DekuError::IdVariantNotFound),
                }
            }
        }
    };
    tokens.extend(deku_id);

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_magic_read(input: &DekuData) -> TokenStream {
    if let Some(magic) = &input.magic {
        quote! {
            let __deku_magic = #magic;

            for __deku_byte in __deku_magic {
                let (__deku_new_rest, __deku_read_byte) = u8::read(__deku_rest, ())?;
                if *__deku_byte != __deku_read_byte {
                    return Err(DekuError::Parse(format!("Missing magic value {:?}", #magic)));
                }

                __deku_rest = __deku_new_rest;
            }
        }
    } else {
        quote! {}
    }
}

struct FieldIdent {
    field_ident: TokenStream,
    is_temp: bool,
}

fn emit_field_reads(
    input: &DekuData,
    fields: &Fields<&FieldData>,
) -> Result<(Vec<FieldIdent>, Vec<TokenStream>), syn::Error> {
    let mut field_reads = vec![];
    let mut field_idents = vec![];

    for (i, f) in fields.iter().enumerate() {
        let (field_ident, field_read) = emit_field_read(input, i, f)?;
        field_idents.push(FieldIdent {
            field_ident,
            is_temp: f.temp,
        });
        field_reads.push(field_read);
    }

    Ok((field_idents, field_reads))
}

fn emit_bit_byte_offsets(
    fields: &[&Option<TokenStream>],
) -> (Option<TokenStream>, Option<TokenStream>) {
    // determine if we should include `bit_offset` and `byte_offset`
    let byte_offset = if fields
        .iter()
        .any(|v| token_contains_string(v, "__deku_byte_offset"))
    {
        Some(quote! {
            let __deku_byte_offset = __deku_bit_offset / 8;
        })
    } else {
        None
    };

    let bit_offset = if fields
        .iter()
        .any(|v| token_contains_string(v, "__deku_bit_offset"))
        || byte_offset.is_some()
    {
        Some(quote! {
            let __deku_bit_offset = usize::try_from(__deku_input_bits.offset_from(__deku_rest))?;
        })
    } else {
        None
    };

    (bit_offset, byte_offset)
}

fn emit_padding(bit_size: &TokenStream) -> TokenStream {
    quote! {
        {
            use core::convert::TryFrom;
            let __deku_pad = usize::try_from(#bit_size).map_err(|e|
                DekuError::InvalidParam(format!(
                    "Invalid padding param \"{}\": cannot convert to usize",
                    stringify!(#bit_size)
                ))
            )?;

            if __deku_rest.len() >= __deku_pad {
                let (__deku_padded_bits, __deku_new_rest) = __deku_rest.split_at(__deku_pad);
                __deku_rest = __deku_new_rest;
            } else {
                return Err(DekuError::Incomplete(NeedSize::new(__deku_pad)));
            }
        }
    }
}

fn emit_field_read(
    input: &DekuData,
    i: usize,
    f: &FieldData,
) -> Result<(TokenStream, TokenStream), syn::Error> {
    let field_type = &f.ty;

    let field_endian = f.endian.as_ref().or_else(|| input.endian.as_ref());

    let field_reader = &f.reader;

    // fields to check usage of bit/byte offset
    let field_check_vars = [
        &f.count,
        &f.bits_read,
        &f.bytes_read,
        &f.until,
        &f.cond,
        &f.default,
        &f.map,
        &f.reader,
        &f.ctx.as_ref().map(|v| quote!(#v)),
        &f.assert,
        &f.assert_eq,
    ];

    let (bit_offset, byte_offset) = emit_bit_byte_offsets(&field_check_vars);

    let field_map = f
        .map
        .as_ref()
        .map(|v| {
            quote! { (#v) }
        })
        .or_else(|| Some(quote! { Result::<_, DekuError>::Ok }));

    let field_ident = f.get_ident(i, true);
    let field_ident_str = field_ident.to_string();
    let internal_field_ident = gen_internal_field_ident(&field_ident);

    let field_assert = f.assert.as_ref().map(|v| {
        quote! {
            if (!(#v)) {
                // assertion is false, raise error
                return Err(DekuError::Assertion(format!(
                            "field '{}' failed assertion: {}",
                            #field_ident_str,
                            stringify!(#v)
                        )));
            } else {
                // do nothing
            }
        }
    });

    let field_assert_eq = f.assert_eq.as_ref().map(|v| {
        quote! {
            if (!(#internal_field_ident == (#v))) {
                // assertion is false, raise error
                return Err(DekuError::Assertion(format!(
                            "field '{}' failed assertion: {}",
                            #field_ident_str,
                            stringify!(#field_ident == #v)
                        )));
            } else {
                // do nothing
            }
        }
    });

    let field_read_func = if field_reader.is_some() {
        quote! { #field_reader }
    } else {
        let read_args = gen_field_args(
            field_endian,
            f.bits.as_ref(),
            f.bytes.as_ref(),
            f.ctx.as_ref(),
        )?;

        // The container limiting options are special, we need to generate `(limit, (other, ..))` for them.
        // These have a problem where when it isn't a copy type, the field will be moved.
        // e.g. struct FooBar {
        //   a: Baz // a type implement `Into<usize>` but not `Copy`.
        //   #[deku(count = "a") <-- Oops, use of moved value: `a`
        //   b: Vec<_>
        // }
        if let Some(field_count) = &f.count {
            quote! {
                {
                    use core::borrow::Borrow;
                    DekuRead::read(__deku_rest, (deku::ctx::Limit::new_count(usize::try_from(*((#field_count).borrow()))?), (#read_args)))
                }
            }
        } else if let Some(field_bits) = &f.bits_read {
            quote! {
                {
                    use core::borrow::Borrow;
                    DekuRead::read(__deku_rest, (deku::ctx::Limit::new_size(deku::ctx::Size::Bits(usize::try_from(*((#field_bits).borrow()))?)), (#read_args)))
                }
            }
        } else if let Some(field_bytes) = &f.bytes_read {
            quote! {
                {
                    use core::borrow::Borrow;
                    DekuRead::read(__deku_rest, (deku::ctx::Limit::new_size(deku::ctx::Size::Bytes(usize::try_from(*((#field_bytes).borrow()))?)), (#read_args)))
                }
            }
        } else if let Some(field_until) = &f.until {
            // We wrap the input into another closure here to enforce that it is actually a callable
            // Otherwise, an incorrectly passed-in integer could unexpectedly convert into a `Count` limit
            quote! {DekuRead::read(__deku_rest, (deku::ctx::Limit::new_until(#field_until), (#read_args)))}
        } else {
            quote! {DekuRead::read(__deku_rest, (#read_args))}
        }
    };

    let pad_bits_before = pad_bits(
        f.pad_bits_before.as_ref(),
        f.pad_bytes_before.as_ref(),
        emit_padding,
    );
    let pad_bits_after = pad_bits(
        f.pad_bits_after.as_ref(),
        f.pad_bytes_after.as_ref(),
        emit_padding,
    );

    let field_read_normal = quote! {
        let (__deku_new_rest, __deku_value) = #field_read_func?;
        let __deku_value: #field_type = #field_map(__deku_value)?;

        __deku_rest = __deku_new_rest;

        __deku_value
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
        #pad_bits_before

        #bit_offset
        #byte_offset

        let #internal_field_ident = {
            #field_read_tokens
        };
        let #field_ident = &#internal_field_ident;

        #field_assert
        #field_assert_eq

        #pad_bits_after
    };

    Ok((field_ident, field_read))
}

/// emit `from_bytes()` for struct/enum
pub fn emit_from_bytes(
    imp: &syn::ImplGenerics,
    lifetime: &TokenStream,
    ident: &TokenStream,
    wher: Option<&syn::WhereClause>,
    body: TokenStream,
) -> TokenStream {
    quote! {
        impl #imp DekuContainerRead<#lifetime> for #ident #wher {
            #[allow(non_snake_case)]
            fn from_bytes(__deku_input: (&#lifetime [u8], usize)) -> Result<((&#lifetime [u8], usize), Self), DekuError> {
                #body
            }
        }
    }
}

/// emit `TryFrom` trait for struct/enum
pub fn emit_try_from(
    imp: &syn::ImplGenerics,
    lifetime: &TokenStream,
    ident: &TokenStream,
    wher: Option<&syn::WhereClause>,
) -> TokenStream {
    quote! {
        impl #imp core::convert::TryFrom<&#lifetime [u8]> for #ident #wher {
            type Error = DekuError;

            fn try_from(input: &#lifetime [u8]) -> Result<Self, Self::Error> {
                let (rest, res) = Self::from_bytes((input, 0))?;
                if !rest.0.is_empty() {
                    return Err(DekuError::Parse(format!("Too much data")));
                }
                Ok(res)
            }
        }
    }
}
