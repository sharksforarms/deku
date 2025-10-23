use std::convert::TryFrom;

use darling::ast::{Data, Fields};
use proc_macro2::TokenStream;
use quote::quote;
#[cfg(feature = "bits")]
use syn::LitStr;

#[cfg(feature = "bits")]
use crate::macros::gen_bit_order_from_str;

use crate::macros::{
    assertion_failed, gen_ctx_types_and_arg, gen_field_args, gen_internal_field_ident,
    gen_struct_destruction, token_contains_string, wrap_default_ctx,
};
use crate::{from_token, DekuData, DekuDataEnum, DekuDataStruct, FieldData, Id};

pub(crate) fn emit_deku_write(input: &DekuData) -> Result<TokenStream, syn::Error> {
    match &input.data {
        Data::Enum(_) => emit_enum(input),
        Data::Struct(_) => emit_struct(input),
    }
}

fn emit_struct(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();
    let mut tokens = TokenStream::new();

    let DekuDataStruct {
        imp,
        wher,
        ident,
        fields,
    } = DekuDataStruct::try_from(input)?;

    let seek = if let Some(num) = &input.seek_from_current {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                let seek_amt = i64::try_from(#num).expect("could not convert into i64");
                if let Err(e) = __deku_writer.seek(SeekFrom::Current(seek_amt)) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if let Some(num) = &input.seek_from_end {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                let seek_amt = i64::try_from(#num).expect("could not convert into i64");
                if let Err(e) = __deku_writer.seek(SeekFrom::End(seek_amt)) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if let Some(num) = &input.seek_from_start {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                let seek_amt = u64::try_from(#num).expect("could not convert into u64");
                if let Err(e) = __deku_writer.seek(SeekFrom::Start(seek_amt)) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if input.seek_rewind {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                if let Err(e) = __deku_writer.rewind() {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else {
        quote! {}
    };

    let magic_write = emit_magic_write(input);

    let field_writes = emit_field_writes(input, &fields, false, None, &ident)?;
    let field_updates = emit_field_updates(&fields, Some(quote! { self. }));

    let named = fields.style.is_struct();
    let unit = fields.style.is_unit();

    let field_idents = fields.iter().enumerate().filter_map(|(i, f)| {
        if !f.temp {
            Some(f.get_ident(i, true))
        } else {
            None
        }
    });

    let destructured = gen_struct_destruction(named, unit, &input.ident, field_idents);

    // Implement `DekuContainerWrite` for types that don't need a context
    if input.ctx.is_none() || (input.ctx.is_some() && input.ctx_default.is_some()) {
        #[cfg(all(feature = "alloc", feature = "bits"))]
        tokens.extend(quote! {
            #[automatically_derived]
             impl #imp core::convert::TryFrom<#ident> for ::#crate_::bitvec::BitVec<u8, ::#crate_::bitvec::Msb0> #wher {
                type Error = ::#crate_::DekuError;

                #[inline]
                fn try_from(input: #ident) -> core::result::Result<Self, Self::Error> {
                    use ::#crate_::DekuContainerWrite as _;
                    input.to_bits()
                }
            }
        });

        #[cfg(feature = "alloc")]
        tokens.extend(quote! {
            const _: () = {
                extern crate alloc;
                use alloc::vec::Vec;

                #[automatically_derived]
                impl #imp core::convert::TryFrom<#ident> for Vec<u8> #wher {
                    type Error = ::#crate_::DekuError;

                    #[inline]
                    fn try_from(input: #ident) -> core::result::Result<Self, Self::Error> {
                        ::#crate_::DekuContainerWrite::to_bytes(&input)
                    }
                }
            };
        });

        tokens.extend(quote! {
            #[automatically_derived]
            impl #imp ::#crate_::DekuContainerWrite for #ident #wher {}
        });
    }

    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    let write_body = quote! {
        match *self {
            #destructured => {
                #seek
                #magic_write
                #(#field_writes)*

                Ok(())
            }
        }
    };

    // avoid outputing `use core::convert::TryInto` if update() function is empty
    let update_use = check_update_use(&field_updates);

    tokens.extend(quote! {
        #[automatically_derived]
        impl #imp ::#crate_::DekuUpdate for #ident #wher {
            #[inline]
            fn update(&mut self) -> core::result::Result<(), ::#crate_::DekuError> {
                #update_use
                #(#field_updates)*

                Ok(())
            }
        }

        #[automatically_derived]
        impl #imp ::#crate_::DekuWriter<#ctx_types> for #ident #wher {
            #[allow(unused_variables)]
            #[inline]
            fn to_writer<W: ::#crate_::no_std_io::Write + ::#crate_::no_std_io::Seek>(&self, __deku_writer: &mut ::#crate_::writer::Writer<W>, #ctx_arg) -> core::result::Result<(), ::#crate_::DekuError> {
                #write_body
            }
        }
    });

    if input.ctx.is_some() && input.ctx_default.is_some() {
        let write_body = wrap_default_ctx(write_body, &input.ctx, &input.ctx_default);

        tokens.extend(quote! {
            #[automatically_derived]
            impl #imp ::#crate_::DekuWriter for #ident #wher {
                #[allow(unused_variables)]
                #[inline]
                fn to_writer<W: ::#crate_::no_std_io::Write + ::#crate_::no_std_io::Seek>(&self, __deku_writer: &mut ::#crate_::writer::Writer<W>, _: ()) -> core::result::Result<(), ::#crate_::DekuError> {
                    #write_body
                }
            }
        });
    }

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_enum(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();
    let mut tokens = TokenStream::new();

    let DekuDataEnum {
        imp,
        wher,
        variants,
        ident,
        id,
        id_type,
        id_args,
    } = DekuDataEnum::try_from(input)?;

    let magic_write = emit_magic_write(input);

    let mut variant_writes = Vec::with_capacity(variants.len());
    let mut variant_updates = Vec::with_capacity(variants.len());

    let has_discriminant = variants.iter().any(|v| v.discriminant.is_some());

    for variant in variants {
        // check if the first field has an ident, if not, it's a unnamed struct
        let variant_is_named = variant
            .fields
            .fields
            .first()
            .and_then(|v| v.ident.as_ref())
            .is_some();

        let variant_ident = &variant.ident;
        let variant_writer = &variant.writer;

        let field_idents = variant.fields.iter().enumerate().filter_map(|(i, f)| {
            if !f.temp {
                Some(f.get_ident(i, true))
            } else {
                None
            }
        });

        let variant_id_write = if id.is_some() {
            quote! {
                // if we don't do this we may get a "unused variable" error if passed via `ctx`
                // i.e. #[deku(ctx = "my_id: u8", id = "my_id")]
                let _ = (#id);
            }
        } else if id_type.is_some() {
            if let Some(variant_id) = &variant.id {
                match variant_id {
                    Id::TokenStream(v) => {
                        quote! {
                            let mut __deku_variant_id: #id_type = #v;
                            __deku_variant_id.to_writer(__deku_writer, (#id_args))?;
                        }
                    }
                    Id::Int(v) => {
                        quote! {
                            let mut __deku_variant_id: #id_type = #v;
                            __deku_variant_id.to_writer(__deku_writer, (#id_args))?;
                        }
                    }
                    Id::Bool(v) => {
                        quote! {
                            let mut __deku_variant_id: #id_type = #v;
                            __deku_variant_id.to_writer(__deku_writer, (#id_args))?;
                        }
                    }
                    Id::LitByteStr(v) => {
                        quote! {
                            let mut __deku_variant_id: #id_type = *#v;
                            __deku_variant_id.to_writer(__deku_writer, (#id_args))?;
                        }
                    }
                }
            } else if variant.id_pat.is_some() && !variant.fields.is_empty() {
                // if the variant has fields, the first must be storing the id
                quote! {}
            } else if has_discriminant {
                match input.repr {
                    None => {
                        return Err(syn::Error::new(
                            variant.ident.span(),
                            "DekuWrite: `id_type` with non-unit variants requires primitive representation i.e. `repr(inttype)`",
                        ));
                    }
                    Some(repr) => {
                        if let Some(id_type) = id_type {
                            if let Some(id_type_repr) = from_token(id_type.clone()) {
                                if id_type_repr != repr {
                                    return Err(syn::Error::new(
                                        variant.ident.span(),
                                        "DekuWrite: `repr` must match `id_type`",
                                    ));
                                }
                            } else {
                                return Err(syn::Error::new(
                                    variant.ident.span(),
                                    "DekuWrite: `repr` must be specified on non-unit variants",
                                ));
                            }
                        }
                    }
                }
                quote! {
                    // https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
                    let mut __deku_variant_id: #id_type = unsafe { *(&Self::#variant_ident as *const Self as *const #id_type) };
                    __deku_variant_id.to_writer(__deku_writer, (#id_args))?;
                }
            } else {
                return Err(syn::Error::new(
                    variant.ident.span(),
                    if variant.id_pat.is_some() && !has_discriminant {
                        "DekuWrite: cannot determine write `id`. must provide storage for the id or discriminant"
                    } else {
                        "DekuWrite: `id` must be specified on non-unit variants"
                    },
                ));
            }
        } else {
            // either `id` or `id_type` needs to be specified
            unreachable!();
        };

        let variant_match = super::gen_enum_init(variant_is_named, variant_ident, field_idents);

        let variant_write = if variant_writer.is_some() {
            quote! { #variant_writer ?; }
        } else {
            let field_writes = emit_field_writes(
                input,
                &variant.fields.as_ref(),
                variant.id_pat.is_some(),
                None,
                &ident,
            )?;

            quote! {
                {
                    #variant_id_write
                    #(#field_writes)*
                }
            }
        };

        let variant_field_updates = emit_field_updates(&variant.fields.as_ref(), None);

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

    // Implement `DekuContainerWrite` for types that don't need a context
    if input.ctx.is_none() || (input.ctx.is_some() && input.ctx_default.is_some()) {
        #[cfg(all(feature = "alloc", feature = "bits"))]
        tokens.extend(quote! {
            #[automatically_derived]
             impl #imp core::convert::TryFrom<#ident> for ::#crate_::bitvec::BitVec<u8, ::#crate_::bitvec::Msb0> #wher {
                type Error = ::#crate_::DekuError;

                #[inline]
                fn try_from(input: #ident) -> core::result::Result<Self, Self::Error> {
                    use ::#crate_::DekuContainerWrite as _;
                    input.to_bits()
                }
            }
        });

        #[cfg(feature = "alloc")]
        tokens.extend(quote! {
            const _: () = {
                extern crate alloc;
                use alloc::vec::Vec;

                #[automatically_derived]
                impl #imp core::convert::TryFrom<#ident> for Vec<u8> #wher {
                    type Error = ::#crate_::DekuError;

                    #[inline]
                    fn try_from(input: #ident) -> core::result::Result<Self, Self::Error> {
                        ::#crate_::DekuContainerWrite::to_bytes(&input)
                    }
                }
            };

            #[automatically_derived]
            impl #imp ::#crate_::DekuContainerWrite for #ident #wher {}
        });
    }

    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    let write_body = quote! {
        #magic_write

        match self {
            #(#variant_writes),*
        }

        Ok(())
    };

    // avoid outputting `use core::convert::TryInto` if update() function is empty
    let update_use = check_update_use(&variant_updates);

    tokens.extend(quote! {
        #[automatically_derived]
        impl #imp ::#crate_::DekuUpdate for #ident #wher {
            #[inline]
            fn update(&mut self) -> core::result::Result<(), ::#crate_::DekuError> {
                #update_use

                match self {
                    #(#variant_updates),*
                }

                Ok(())
            }
        }

        #[automatically_derived]
        impl #imp ::#crate_::DekuWriter<#ctx_types> for #ident #wher {
            #[allow(unused_variables)]
            #[inline]
            fn to_writer<W: ::#crate_::no_std_io::Write + ::#crate_::no_std_io::Seek>(&self, __deku_writer: &mut ::#crate_::writer::Writer<W>, #ctx_arg) -> core::result::Result<(), ::#crate_::DekuError> {
                #write_body
            }
        }
    });

    if input.ctx.is_some() && input.ctx_default.is_some() {
        let write_body = wrap_default_ctx(write_body, &input.ctx, &input.ctx_default);

        tokens.extend(quote! {
            #[automatically_derived]
            impl #imp ::#crate_::DekuWriter for #ident #wher {
                #[allow(unused_variables)]
                #[inline]
                fn to_writer<W: ::#crate_::no_std_io::Write + ::#crate_::no_std_io::Seek>(&self, __deku_writer: &mut ::#crate_::writer::Writer<W>, _: ()) -> core::result::Result<(), ::#crate_::DekuError> {
                    #write_body
                }
            }
        });
    }

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_magic_write(input: &DekuData) -> TokenStream {
    let crate_ = super::get_crate_name();
    if let Some(magic) = &input.magic {
        quote! {
            ::#crate_::DekuWriter::to_writer(#magic, __deku_writer, ())?;
        }
    } else {
        quote! {}
    }
}

fn emit_field_writes(
    input: &DekuData,
    fields: &Fields<&FieldData>,
    is_id_pat: bool,
    object_prefix: Option<TokenStream>,
    ident: &TokenStream,
) -> Result<Vec<TokenStream>, syn::Error> {
    let mut is_id_pat = is_id_pat;
    fields
        .iter()
        .enumerate()
        .map(|(i, f)| emit_field_write(input, i, f, &object_prefix, ident, &mut is_id_pat))
        .collect()
}

fn emit_field_updates(
    fields: &Fields<&FieldData>,
    object_prefix: Option<TokenStream>,
) -> Vec<TokenStream> {
    fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| emit_field_update(i, f, &object_prefix))
        .collect()
}

fn emit_field_update(
    i: usize,
    f: &FieldData,
    object_prefix: &Option<TokenStream>,
) -> Option<TokenStream> {
    if f.temp {
        return None;
    }
    let field_ident = f.get_ident(i, object_prefix.is_none());
    let deref = if object_prefix.is_none() {
        Some(quote! { * })
    } else {
        None
    };

    f.update.as_ref().map(|field_update| {
        quote! {
            #deref #object_prefix #field_ident = (#field_update).try_into()?;
        }
    })
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
            let __deku_byte_offset = __deku_writer.bits_written / 8;
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
            let __deku_bit_offset = __deku_writer.bits_written;
        })
    } else {
        None
    };

    (bit_offset, byte_offset)
}

#[cfg(feature = "bits")]
fn emit_padding(bit_size: &TokenStream, bit_order: Option<&LitStr>) -> TokenStream {
    let crate_ = super::get_crate_name();
    const PAD: usize = crate::PAD_ARRAY_SIZE * 8;
    if let Some(bit_order) = bit_order {
        let order = gen_bit_order_from_str(bit_order).unwrap();
        quote! {
            {
                use core::convert::TryFrom;
                let mut __deku_pad = usize::try_from(#bit_size).map_err(|e|
                    ::#crate_::deku_error!(::#crate_::DekuError::InvalidParam, "Invalid padding param, cannot convert to usize", "{}", stringify!(#bit_size))
                )?;
                let __deku_pad_source = ::#crate_::bitvec::bitarr!(u8, ::#crate_::bitvec::Msb0; 0; #PAD);
                while __deku_pad > 0 {
                    let __deku_pad_chunk = core::cmp::min(__deku_pad_source.len(), __deku_pad);
                    __deku_writer.write_bits_order(&__deku_pad_source[..__deku_pad_chunk], #order)?;
                    __deku_pad -= __deku_pad_chunk;
                }
            }
        }
    } else {
        quote! {
            {
                use core::convert::TryFrom;
                let mut __deku_pad = usize::try_from(#bit_size).map_err(|e|
                    ::#crate_::deku_error!(::#crate_::DekuError::InvalidParam, "Invalid padding param, cannot convert to usize", "{}", stringify!(#bit_size))
                )?;
                let __deku_pad_source = ::#crate_::bitvec::bitarr!(u8, ::#crate_::bitvec::Msb0; 0; #PAD);
                while __deku_pad > 0 {
                    let __deku_pad_chunk = core::cmp::min(__deku_pad_source.len(), __deku_pad);
                    __deku_writer.write_bits(&__deku_pad_source[..__deku_pad_chunk])?;
                    __deku_pad -= __deku_pad_chunk;
                }
            }
        }
    }
}

// TODO: if this is a simple calculation such as "8 + 2", this could be const
#[cfg(not(feature = "bits"))]
fn emit_padding_bytes(bit_size: &TokenStream) -> TokenStream {
    let crate_ = super::get_crate_name();
    let pad = crate::PAD_ARRAY_SIZE;
    quote! {
        {
            use core::convert::TryFrom;
            let mut __deku_pad = usize::try_from(#bit_size).map_err(|e|
                ::#crate_::deku_error!(::#crate_::DekuError::InvalidParam, "Invalid padding param, cannot convert to usize", "{}", stringify!(#bit_size))
            )?;

            let __deku_pad_source = [0u8; #pad];
            while __deku_pad > 0 {
                let __deku_pad_chunk = core::cmp::min(__deku_pad_source.len(), __deku_pad);
                __deku_writer.write_bytes(&__deku_pad_source[..__deku_pad_chunk])?;
                __deku_pad -= __deku_pad_chunk;
            }
        }
    }
}

fn emit_field_write(
    input: &DekuData,
    i: usize,
    f: &FieldData,
    object_prefix: &Option<TokenStream>,
    ident: &TokenStream,
    is_id_pat: &mut bool,
) -> Result<TokenStream, syn::Error> {
    // If this is the storage field of the Enum id_pat, we use
    // the field args that were used for reading, to ensure that
    // the writing will be the same
    if *is_id_pat {
        // TODO: This should error on some attributes that don't make sense aren't used?
        // Such as magic, seek*
        let crate_ = super::get_crate_name();
        let field_endian = input.id_endian.as_ref();
        #[cfg(feature = "bits")]
        let field_bits = input.bits.as_ref();
        #[cfg(not(feature = "bits"))]
        let field_bits = None;
        let field_bytes = input.bytes.as_ref();
        let field_bit_order = input.bit_order.as_ref();
        let field_ident = f.get_ident(i, object_prefix.is_none());

        let write_args =
            gen_field_args(field_endian, field_bits, field_bytes, None, field_bit_order)?;

        let ret = quote! {
            ::#crate_::DekuWriter::to_writer(#object_prefix #field_ident, __deku_writer, (#write_args))?;
        };

        // only the first field
        *is_id_pat = false;
        return Ok(ret);
    }

    let crate_ = super::get_crate_name();
    let field_endian = f.endian.as_ref().or(input.endian.as_ref());
    let field_bit_order = f.bit_order.as_ref().or(input.bit_order.as_ref());
    let magic_write = if let Some(magic) = &f.magic {
        quote! {
            ::#crate_::DekuWriter::to_writer(#magic, __deku_writer, ())?;
        }
    } else {
        quote! {}
    };

    let seek = if let Some(num) = &f.seek_from_current {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                if let Err(e) = __deku_writer.seek(SeekFrom::Current(i64::try_from(#num).unwrap())) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if let Some(num) = &f.seek_from_end {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                if let Err(e) = __deku_writer.seek(SeekFrom::End(i64::try_from(#num).unwrap())) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if let Some(num) = &f.seek_from_start {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                if let Err(e) = __deku_writer.seek(SeekFrom::Start(u64::try_from(#num).unwrap())) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if f.seek_rewind {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                if let Err(e) = __deku_writer.rewind() {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else {
        quote! {}
    };

    // fields to check usage of bit/byte offset
    let field_check_vars = [
        &f.writer,
        &f.cond,
        &f.ctx.as_ref().map(|v| quote!(#v)),
        &f.assert,
        &f.assert_eq,
    ];

    let (bit_offset, byte_offset) = emit_bit_byte_offsets(&field_check_vars);

    let ident = &ident.to_string();
    let field_writer = &f.writer;
    let field_ident = f.get_ident(i, object_prefix.is_none());
    let field_ident_str = field_ident.to_string();

    let field_assert = f.assert.as_ref().map(|v| {
        let return_error = assertion_failed(v, ident, &field_ident_str, None);
        quote! {
            if (!(#v)) {
                #return_error
            } else {
                // do nothing
            }
        }
    });

    let field_assert_eq = f.assert_eq.as_ref().map(|v| {
        let return_error = assertion_failed(v, ident, &field_ident_str, Some(&field_ident));
        quote! {
            if (!(*(#field_ident) == (#v))) {
                #return_error
            } else {
                // do nothing
            }
        }
    });

    let trace_field_log = if cfg!(feature = "logging") {
        quote! {
            log::trace!("Writing: {}.{}", #ident, #field_ident_str);
        }
    } else {
        quote! {}
    };

    let field_write_func = if field_writer.is_some() {
        quote! { #field_writer }
    } else {
        let write_args = gen_field_args(
            field_endian,
            #[cfg(feature = "bits")]
            f.bits.as_ref(),
            #[cfg(not(feature = "bits"))]
            None,
            f.bytes.as_ref(),
            f.ctx.as_ref(),
            field_bit_order,
        )?;

        if f.temp {
            if f.temp_value.is_some() {
                quote! {
                    ::#crate_::DekuWriter::to_writer(#object_prefix &#field_ident, __deku_writer, (#write_args))
                }
            } else {
                quote! { core::result::Result::<(), ::#crate_::DekuError>::Ok(()) }
            }
        } else {
            quote! { ::#crate_::DekuWriter::to_writer(#object_prefix #field_ident, __deku_writer, (#write_args)) }
        }
    };

    #[cfg(feature = "bits")]
    let pad_bits_before = crate::macros::pad_bits(
        f.pad_bits_before.as_ref(),
        f.pad_bytes_before.as_ref(),
        field_bit_order,
        emit_padding,
    );
    #[cfg(feature = "bits")]
    let pad_bits_after = crate::macros::pad_bits(
        f.pad_bits_after.as_ref(),
        f.pad_bytes_after.as_ref(),
        field_bit_order,
        emit_padding,
    );

    #[cfg(not(feature = "bits"))]
    let pad_bits_before = crate::macros::pad_bytes(f.pad_bytes_before.as_ref(), emit_padding_bytes);

    #[cfg(not(feature = "bits"))]
    let pad_bits_after = crate::macros::pad_bytes(f.pad_bytes_after.as_ref(), emit_padding_bytes);

    let field_write_normal = quote! {
        #field_write_func ?;
    };

    let skipping_log = if cfg!(feature = "logging") {
        quote! {
            log::trace!("skipping");
        }
    } else {
        quote! {}
    };

    let temp_decl = if f.temp {
        if let Some(temp_value) = &f.temp_value {
            let field_type = &f.ty;
            let internal_field_ident = gen_internal_field_ident(&field_ident);
            Some(quote! {
                let #internal_field_ident: #field_type = #temp_value;
                let #field_ident: &#field_type = &#internal_field_ident;
            })
        } else {
            None // Note: temp w/o temp_value with not work
        }
    } else {
        None
    };
    let field_write_tokens = match (f.skip, &f.cond) {
        (true, Some(field_cond)) => {
            // #[deku(skip, cond = "...")] ==> `skip` if `cond`
            quote! {
                #temp_decl
                if (#field_cond) {
                    #skipping_log
                   // skipping, no write
                } else {
                    #field_write_normal
                }
            }
        }
        (true, None) => {
            // #[deku(skip)] ==> `skip`
            quote! {
                #skipping_log
                // skipping, no write
            }
        }
        (false, _) => {
            quote! {
                #temp_decl
                #field_write_normal
            }
        }
    };

    let field_write = quote! {
        #seek
        #magic_write
        #pad_bits_before

        #bit_offset
        #byte_offset

        #trace_field_log
        #field_assert
        #field_assert_eq

        #field_write_tokens

        #pad_bits_after
    };

    Ok(field_write)
}

/// avoid outputing `use core::convert::TryInto` if update() function is generated with empty Vec
fn check_update_use<T>(vec: &[T]) -> TokenStream {
    if !vec.is_empty() {
        quote! {use core::convert::TryInto;}
    } else {
        quote! {}
    }
}
