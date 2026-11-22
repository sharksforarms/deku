use std::convert::TryFrom;

use darling::ToTokens;
use darling::ast::{Data, Fields};
use proc_macro2::TokenStream;
use quote::quote;
#[cfg(feature = "bits")]
use syn::LitStr;
use syn::{Ident, LitByteStr};

#[cfg(feature = "bits")]
use crate::macros::gen_bit_order_from_str;

use crate::macros::{
    assertion_failed, gen_ctx_types_and_arg, gen_field_args, gen_internal_field_idents,
    token_contains_string, wrap_default_ctx,
};
use crate::{DekuData, DekuDataEnum, DekuDataStruct, FieldData, Id, from_token};

#[cfg(feature = "bits")]
use super::{
    BitRunField, batch_condition, bit_field_bound, is_bool, needs_fallback, run_field, sum_bits,
};
use super::{gen_internal_field_ident, gen_type_from_ctx_id};

pub(crate) fn emit_deku_read(input: &DekuData) -> Result<TokenStream, syn::Error> {
    match &input.data {
        Data::Enum(_) => emit_enum(input),
        Data::Struct(_) => emit_struct(input),
    }
}

fn emit_struct(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();
    let mut tokens = TokenStream::new();

    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map_or(quote!('_), |v| quote!(#v));

    let wher = input.read_where();
    let wher = wher.as_deref();

    let DekuDataStruct { imp, ident, fields } = DekuDataStruct::try_from(input)?;

    let seek = if let Some(num) = &input.seek_from_current {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                let seek_amt = i64::try_from(#num).expect("could not convert into i64");
                if let Err(e) = __deku_reader.seek(SeekFrom::Current(seek_amt)) {
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
                if let Err(e) = __deku_reader.seek(SeekFrom::End(seek_amt)) {
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
                if let Err(e) = __deku_reader.seek(SeekFrom::Start(seek_amt)) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if input.seek_rewind {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                if let Err(e) = __deku_reader.rewind() {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else {
        quote! {}
    };

    let magic_read = emit_magic_read(input);

    // check if the first field has an ident, if not, it's a unnamed struct
    let is_named_struct = fields
        .fields
        .first()
        .and_then(|v| v.ident.as_ref())
        .is_some();

    let (field_idents, field_reads) = emit_field_reads(input, &fields, &ident, false)?;

    // filter out temporary fields
    let field_idents = field_idents
        .iter()
        .filter(|f| !f.is_temp)
        .map(|f| &f.field_ident);

    let internal_fields = gen_internal_field_idents(is_named_struct, field_idents);

    let initialize_struct = super::gen_struct_init(is_named_struct, internal_fields);

    // Implement `DekuContainerRead` for types that don't need a context
    if input.ctx.is_none() || (input.ctx.is_some() && input.ctx_default.is_some()) {
        let from_reader_body = quote! {
            use core::convert::TryFrom;
            use ::#crate_::DekuReader as _;
            let __deku_reader = &mut ::#crate_::reader::Reader::new(__deku_input.0);
            if __deku_input.1 != 0 {
                __deku_reader.skip_bits(__deku_input.1, ::#crate_::ctx::Order::default())?;
            }

            let __deku_value = Self::from_reader_with_ctx(__deku_reader, ())?;

            Ok((__deku_reader.bits_read, __deku_value))
        };

        let from_bytes_body = quote! {
            use core::convert::TryFrom;
            use ::#crate_::DekuReader as _;
            let mut __deku_cursor = #crate_::no_std_io::Cursor::new(__deku_input.0);
            let mut __deku_reader = &mut ::#crate_::reader::Reader::new(&mut __deku_cursor);
            if __deku_input.1 != 0 {
                __deku_reader.skip_bits(__deku_input.1, ::#crate_::ctx::Order::default())?;
            }

            let __deku_value = Self::from_reader_with_ctx(__deku_reader, ())?;
            let read_whole_byte = (__deku_reader.bits_read % 8) == 0;
            let idx = if read_whole_byte {
                __deku_reader.bits_read / 8
            } else {
                (__deku_reader.bits_read - (__deku_reader.bits_read % 8)) / 8
            };
            let Some(rest) = __deku_input.0.get(idx..) else {
                return Err(::#crate_::DekuError::Incomplete(::#crate_::prelude::NeedSize::new(8 * (idx - __deku_input.0.len()))));
            };
            Ok(((rest, __deku_reader.bits_read % 8), __deku_value))
        };

        tokens.extend(emit_try_from(&imp, &lifetime, &ident, wher));

        tokens.extend(emit_container_read(
            &imp,
            &lifetime,
            &ident,
            wher,
            from_reader_body,
            from_bytes_body,
        ));
    }

    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    let read_body = quote! {
        use core::convert::TryFrom;

        #seek

        #magic_read

        #(#field_reads)*
        let __deku_value = #initialize_struct;

        Ok(__deku_value)
    };

    tokens.extend(quote! {
        #[automatically_derived]
        impl #imp ::#crate_::DekuReader<#lifetime, #ctx_types> for #ident #wher {
            #[inline]
            fn from_reader_with_ctx<R: ::#crate_::no_std_io::Read + ::#crate_::no_std_io::Seek>(__deku_reader: &mut ::#crate_::reader::Reader<R>, #ctx_arg) -> core::result::Result<Self, ::#crate_::DekuError> {
                #read_body
            }
        }
    });

    if input.ctx.is_some() && input.ctx_default.is_some() {
        let read_body = wrap_default_ctx(read_body, &input.ctx, &input.ctx_default);

        tokens.extend(quote! {
            #[automatically_derived]
            impl #imp ::#crate_::DekuReader<#lifetime> for #ident #wher {
                #[inline]
                fn from_reader_with_ctx<R: ::#crate_::no_std_io::Read + ::#crate_::no_std_io::Seek>(__deku_reader: &mut ::#crate_::reader::Reader<R>, _: ()) -> core::result::Result<Self, ::#crate_::DekuError> {
                    #read_body
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

    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map_or(quote!('_), |v| quote!(#v));

    let ident_as_string = ident.to_string();

    let magic_read = emit_magic_read(input);

    let mut has_default_match = false;
    let mut default_reader = None;
    let mut pre_match_tokens = Vec::with_capacity(variants.len());
    let mut variant_matches = Vec::with_capacity(variants.len());
    let mut deku_ids = Vec::with_capacity(variants.len());

    let has_discriminant = variants.iter().any(|v| v.discriminant.is_some());

    for variant in variants {
        // check if the first field has an ident, if not, it's a unnamed struct
        let variant_is_named = variant
            .fields
            .fields
            .first()
            .and_then(|v| v.ident.as_ref())
            .is_some();

        let mut pad_id = false;
        let variant_id = if let Some(variant_id) = &variant.id {
            match variant_id {
                Id::TokenStream(v) => quote! {&#v}.into_token_stream(),
                Id::LitByteStr(v) => v.into_token_stream(),
                Id::Int(v) => v.into_token_stream(),
                Id::Bool(v) => v.into_token_stream(),
            }
        } else if let Some(variant_id_pat) = &variant.id_pat {
            // If user has supplied an id, then we have an id_pat that and the id variant doesn't
            // need read into an id value
            if id.is_none() {
                pad_id = true;
                variant_id_pat.clone()
            } else {
                variant_id_pat.clone()
            }
        } else if has_discriminant {
            let Some(repr) = input.repr else {
                return Err(syn::Error::new(
                    variant.ident.span(),
                    "DekuRead: `id_type` with non-unit variants requires primitive representation i.e. `repr(inttype)`",
                ));
            };
            if let Some(id_type) = id_type {
                let Some(id_type_repr) = from_token(id_type.clone()) else {
                    return Err(syn::Error::new(
                        variant.ident.span(),
                        "DekuRead: `repr` must be specified on non-unit variants",
                    ));
                };
                if id_type_repr != repr {
                    return Err(syn::Error::new(
                        variant.ident.span(),
                        "DekuRead: `repr` must match `id_type`",
                    ));
                }
            }
            let repr_type: TokenStream = repr.into();
            let ident = &variant.ident;
            let internal_ident = gen_internal_field_ident(&quote!(#ident));
            pre_match_tokens.push(quote! {
                // https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
                let #internal_ident = unsafe { *(&Self::#ident as *const Self as *const #repr_type) };
            });
            quote! { _ if __deku_variant_id == #internal_ident }
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
        let variant_has_default = variant.default.unwrap_or(false);

        let variant_read_func = if variant_reader.is_some() {
            quote! { #variant_reader; }
        } else {
            let (field_idents, field_reads) =
                emit_field_reads(input, &variant.fields.as_ref(), &ident, pad_id)?;

            // filter out temporary fields
            let field_idents = field_idents
                .iter()
                .filter(|f| !f.is_temp)
                .map(|f| &f.field_ident);
            let internal_fields = gen_internal_field_idents(variant_is_named, field_idents);
            let initialize_enum =
                super::gen_enum_init(variant_is_named, variant_ident, internal_fields);

            if let Some(variant_id) = &variant.id {
                let deref = match variant_id {
                    Id::TokenStream(_) => quote! {},
                    Id::Int(_) => quote! {},
                    Id::Bool(_) => quote! {},
                    Id::LitByteStr(_) => quote! {*},
                };

                let deku_id = quote! { Self :: #initialize_enum => Ok(#deref #variant_id)};
                deku_ids.push(deku_id);
            }

            quote! {
                {
                    #(#field_reads)*
                    Self :: #initialize_enum
                }
            }
        };

        // register `default`
        if default_reader.is_some() && variant_has_default {
            return Err(syn::Error::new(
                variant.ident.span(),
                "DekuRead: `default` must be specified only once",
            ));
        } else if default_reader.is_none() && variant_has_default {
            default_reader = Some(variant_read_func.clone())
        }

        variant_matches.push(quote! {
            #variant_id => {
                #variant_read_func
            }
        });
    }

    // if no default match, return error
    if !has_default_match && default_reader.is_none() {
        let no_variant = super::no_variant_error(&crate_, &ident_as_string);
        variant_matches.push(quote! {
            _ => {
                return Err(#no_variant);
            }
        });
    }

    // if default
    if !has_default_match && let Some(variant_read_func) = default_reader {
        variant_matches.push(quote! {
            _ => {
                #variant_read_func
            }
        });
    }

    let variant_id_read = if id.is_some() {
        quote! {
            let __deku_variant_id = (#id);
        }
    } else if let Some(bits) = super::const_id_bits(input) {
        // The read `(Endian, BitSize)` makes of a big-endian unsigned id, with
        // the width a constant, as a bit run reads.
        quote! {
            let __deku_variant_id = __deku_reader.read_bits_uint_msb0_const::<#bits>()? as #id_type;
        }
    } else if id_type.is_some() {
        quote! {
            let __deku_variant_id = <#id_type>::from_reader_with_ctx(__deku_reader,  (#id_args))?;
        }
    } else {
        // either `id` or `id_type` needs to be specified
        unreachable!();
    };

    let unbatched = quote! {
        #variant_id_read

        #(#pre_match_tokens)*

        let __deku_value = match &__deku_variant_id {
            #(#variant_matches),*
        };
    };
    #[cfg(feature = "bits")]
    let fused = if has_discriminant {
        None
    } else {
        emit_fused_enum_read(input)
    };
    #[cfg(not(feature = "bits"))]
    let fused: Option<()> = None;
    let variant_read = match fused {
        #[cfg(feature = "bits")]
        Some(FusedEnumRead { consts, read }) => quote! {
            #consts
            let __deku_value = if __DEKU_FUSED {
                #read
            } else {
                #unbatched
                __deku_value
            };
        },
        _ => unbatched,
    };

    // Implement `DekuContainerRead` for types that don't need a context
    if input.ctx.is_none() || (input.ctx.is_some() && input.ctx_default.is_some()) {
        let from_reader_body = quote! {
            use core::convert::TryFrom;
            use ::#crate_::DekuReader as _;
            let __deku_reader = &mut ::#crate_::reader::Reader::new(__deku_input.0);
            if __deku_input.1 != 0 {
                __deku_reader.skip_bits(__deku_input.1, ::#crate_::ctx::Order::default())?;
            }

            let __deku_value = Self::from_reader_with_ctx(__deku_reader, ())?;

            Ok((__deku_reader.bits_read, __deku_value))
        };

        let from_bytes_body = quote! {
            use core::convert::TryFrom;
            use ::#crate_::DekuReader as _;
            let mut __deku_cursor = #crate_::no_std_io::Cursor::new(__deku_input.0);
            let mut __deku_reader = &mut ::#crate_::reader::Reader::new(&mut __deku_cursor);
            if __deku_input.1 != 0 {
                __deku_reader.skip_bits(__deku_input.1, ::#crate_::ctx::Order::default())?;
            }

            let __deku_value = Self::from_reader_with_ctx(__deku_reader, ())?;
            let read_whole_byte = (__deku_reader.bits_read % 8) == 0;
            let idx = if read_whole_byte {
                __deku_reader.bits_read / 8
            } else {
                (__deku_reader.bits_read - (__deku_reader.bits_read % 8)) / 8
            };
            let Some(rest) = __deku_input.0.get(idx..) else {
                return Err(::#crate_::DekuError::Incomplete(::#crate_::prelude::NeedSize::new(8 * (idx - __deku_input.0.len()))));
            };
            Ok(((rest, __deku_reader.bits_read % 8), __deku_value))
        };

        tokens.extend(emit_try_from(&imp, &lifetime, &ident, wher));

        tokens.extend(emit_container_read(
            &imp,
            &lifetime,
            &ident,
            wher,
            from_reader_body,
            from_bytes_body,
        ));
    }
    let (ctx_types, ctx_arg) = gen_ctx_types_and_arg(input.ctx.as_ref())?;

    let read_body = quote! {
        use core::convert::TryFrom;
        use ::#crate_::DekuReader as _;

        #magic_read

        #variant_read

        Ok(__deku_value)
    };

    tokens.extend(quote! {
        #[allow(non_snake_case)]
        #[automatically_derived]
        impl #imp ::#crate_::DekuReader<#lifetime, #ctx_types> for #ident #wher {
            #[inline]
            fn from_reader_with_ctx<R: ::#crate_::no_std_io::Read + ::#crate_::no_std_io::Seek>(__deku_reader: &mut ::#crate_::reader::Reader<R>, #ctx_arg) -> core::result::Result<Self, ::#crate_::DekuError> {
                #read_body
            }
        }
    });

    if input.ctx.is_some() && input.ctx_default.is_some() {
        let read_body = wrap_default_ctx(read_body, &input.ctx, &input.ctx_default);

        tokens.extend(quote! {
            #[allow(non_snake_case)]
            #[automatically_derived]
            impl #imp ::#crate_::DekuReader<#lifetime> for #ident #wher {
                #[inline]
                fn from_reader_with_ctx<R: ::#crate_::no_std_io::Read + ::#crate_::no_std_io::Seek>(__deku_reader: &mut ::#crate_::reader::Reader<R>, _: ()) -> core::result::Result<Self, ::#crate_::DekuError> {
                    #read_body
                }
            }
        });
    }

    let deku_id_type = if let Some(id_type) = id_type {
        Some(quote! {#id_type})
    } else if let (Some(ctx), Some(id)) = (input.ctx.as_ref(), input.id.as_ref()) {
        gen_type_from_ctx_id(ctx, id)
    } else {
        None
    };

    // Implement `DekuEnumExt`
    if let Some(deku_id_type) = deku_id_type {
        if !imp.to_token_stream().is_empty() {
            // Generics (#imp) are not supported, as our __deku
            // would need to be appended to #imp
        } else {
            tokens.extend(quote! {
            #[automatically_derived]
            impl<'__deku> #imp ::#crate_::DekuEnumExt<#lifetime, (#deku_id_type)> for #ident #wher {
                #[inline]
                fn deku_id(&self) -> core::result::Result<(#deku_id_type), ::#crate_::DekuError> {
                    match self {
                        #(#deku_ids ,)*
                        _ => Err(::#crate_::DekuError::IdVariantNotFound),
                    }
                }
            }
        });
        }
    }

    // println!("{}", tokens.to_string());
    Ok(tokens)
}

fn emit_magic_read(input: &DekuData) -> TokenStream {
    let crate_ = super::get_crate_name();
    if let Some(magic) = &input.magic {
        emit_magic_read_lit(&crate_, magic)
    } else {
        quote! {}
    }
}

fn emit_magic_read_lit(crate_: &Ident, magic: &LitByteStr) -> TokenStream {
    quote! {
        let __deku_magic = #magic;

        for __deku_byte in __deku_magic {
            let __deku_read_byte = u8::from_reader_with_ctx(__deku_reader, ())?;
            if *__deku_byte != __deku_read_byte {
                return Err(::#crate_::deku_error!(::#crate_::DekuError::Parse, "Missing magic value", "{:?}", #magic));
            }
        }
    }
}

struct FieldIdent {
    field_ident: TokenStream,
    is_temp: bool,
}

fn emit_field_reads(
    input: &DekuData,
    fields: &Fields<&FieldData>,
    ident: &TokenStream,
    use_id: bool,
) -> Result<(Vec<FieldIdent>, Vec<TokenStream>), syn::Error> {
    let mut field_reads = Vec::with_capacity(fields.len());
    let mut field_idents = Vec::with_capacity(fields.len());

    let mut use_id = use_id;

    #[cfg(feature = "bits")]
    let runs = super::plan_bit_runs(input, fields, use_id);
    #[cfg(feature = "bits")]
    let taken = |i: usize| runs.get(&i).map(Vec::len);
    #[cfg(not(feature = "bits"))]
    let taken = |_: usize| None;
    let byte_runs = super::plan_byte_runs(input, fields, use_id, &taken);
    #[cfg(feature = "bits")]
    let lsb_runs = super::plan_lsb_runs(input, fields, use_id);

    let mut i = 0;
    while i < fields.len() {
        #[cfg(feature = "bits")]
        if let Some(run) = runs.get(&i) {
            let (idents, read) = emit_bit_run_read(input, fields, i, run, ident)?;
            for field_ident in idents {
                field_idents.push(FieldIdent {
                    field_ident,
                    is_temp: false,
                });
            }
            field_reads.push(read);
            i += run.len();
            use_id = false;
            continue;
        }

        #[cfg(feature = "bits")]
        if let Some(run) = lsb_runs.get(&i) {
            let (idents, read) = emit_lsb_run_read(input, fields, i, run, ident)?;
            for field_ident in idents {
                field_idents.push(FieldIdent {
                    field_ident,
                    is_temp: false,
                });
            }
            field_reads.push(read);
            i += run.len();
            use_id = false;
            continue;
        }

        if let Some(run) = byte_runs.get(&i) {
            let (idents, read) = emit_byte_run_read(fields, i, run, ident)?;
            for field_ident in idents {
                field_idents.push(FieldIdent {
                    field_ident,
                    is_temp: false,
                });
            }
            field_reads.push(read);
            i += run.fields.len();
            use_id = false;
            continue;
        }

        let f = fields.fields[i];
        #[cfg(feature = "bits")]
        field_reads.push(bit_field_bound(f));
        let (field_ident, field_read) = emit_field_read(input, i, f, ident, use_id)?;
        use_id = false;
        field_idents.push(FieldIdent {
            field_ident,
            is_temp: f.temp,
        });
        field_reads.push(field_read);
        i += 1;
    }

    Ok((field_idents, field_reads))
}

/// `N` for a field that is a plain `[u8; N]`, which one `read_exact` can serve,
/// or `None` for anything else.
///
/// The generic `[T; N]` impl reads element by element, with the `MaybeUninit`
/// and drop-on-error bookkeeping the general case needs. A byte array needs
/// none of it: the bytes are the value.
pub(crate) fn byte_array_len(input: &DekuData, f: &FieldData) -> Option<usize> {
    // Anything else moves the cursor, makes the read conditional, or changes
    // what the field means. Vetted on `FieldData`, so a new attribute cannot
    // silently qualify.
    if f.any_field_set_incompatible_with_byte_array() {
        return None;
    }

    // `Msb0` only: on an unaligned cursor `read_bytes_const_into` reverses the
    // buffer for `Lsb0`. A byte has no byte order, so endianness needs no check.
    #[cfg(feature = "bits")]
    if let Some(order) = f.bit_order.as_ref().or(input.bit_order.as_ref())
        && order.value() != "msb"
    {
        return None;
    }
    #[cfg(not(feature = "bits"))]
    let _ = input;

    let syn::Type::Array(array) = &f.ty else {
        return None;
    };
    let syn::Type::Path(elem) = &*array.elem else {
        return None;
    };
    if !elem.path.is_ident("u8") {
        return None;
    }
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(len),
        ..
    }) = &array.len
    else {
        return None;
    };
    let len = len.base10_parse::<usize>().ok()?;
    (len != 0).then_some(len)
}

/// A little-endian `Lsb0` run of whole bytes: where no bits are left over,
/// one read of a little-endian integer, and each field from its bits, the
/// first field lowest. Off a byte boundary, the fields' own reads.
#[cfg(feature = "bits")]
fn emit_lsb_run_read(
    input: &DekuData,
    fields: &Fields<&FieldData>,
    start: usize,
    run: &[super::LsbRunField],
    ident: &TokenStream,
) -> Result<(Vec<TokenStream>, TokenStream), syn::Error> {
    let crate_ = super::get_crate_name();
    let total: usize = run.iter().map(|field| field.bits).sum();
    let bytes = total / 8;
    let run_ident = quote::format_ident!("__deku_lsb_run_{}", start);
    let assertions = run_assertions(fields, start, run.len(), &ident.to_string());

    let mut idents = Vec::with_capacity(run.len());
    let mut internals = Vec::with_capacity(run.len());
    let mut extracts = Vec::with_capacity(run.len());
    let mut fallback = TokenStream::new();
    let mut shift = 0usize;
    for (offset, field) in run.iter().enumerate() {
        let f = fields.fields[start + offset];
        let field_ident = f.get_ident(start + offset, true);
        let internal = gen_internal_field_ident(&field_ident);
        let ty = &field.ty;
        let mask = super::low_mask(field.bits);
        extracts.push(if is_bool(ty) {
            quote! { ((#run_ident >> #shift) & 1) != 0 }
        } else {
            quote! { ((#run_ident >> #shift) & #mask) as #ty }
        });
        let (_, read) = emit_field_read(input, start + offset, f, ident, false)?;
        fallback.extend(read);
        idents.push(field_ident);
        internals.push(internal);
        shift += field.bits;
    }
    let read = quote! {
        let (#(#internals,)*) = if __deku_reader.leftover.is_none() {
            let mut __deku_lsb_bytes = [0u8; 8];
            __deku_reader.read_bytes_const_into::<#bytes>(
                (&mut __deku_lsb_bytes[..#bytes]).try_into().expect("a run of whole bytes"),
                ::#crate_::ctx::Order::Lsb0,
            )?;
            let #run_ident = u64::from_le_bytes(__deku_lsb_bytes);
            (#(#extracts,)*)
        } else {
            #fallback
            (#(#internals,)*)
        };
        #(let #idents = &#internals;)*
        #assertions
    };
    Ok((idents, read))
}

/// One byte read for the whole run, then each field from its own bytes, in
/// the run's byte order. The bytes and the values are those the fields'
/// own reads give, and the one difference is on input too short for the
/// whole run: the run reports `Incomplete` for all of it, and reads none.
fn emit_byte_run_read(
    fields: &Fields<&FieldData>,
    start: usize,
    run: &super::ByteRun,
    ident: &TokenStream,
) -> Result<(Vec<TokenStream>, TokenStream), syn::Error> {
    let crate_ = super::get_crate_name();
    let total: usize = run.fields.iter().map(|f| f.bytes).sum();
    let buffer = quote::format_ident!("__deku_byte_run_{}", start);
    let endian = quote::format_ident!("__deku_byte_run_endian_{}", start);
    let endian_value = match &run.endian {
        Some(endian) => super::gen_endian_from_str(endian)?,
        None => quote! { ::#crate_::ctx::Endian::default() },
    };
    let assertions = run_assertions(fields, start, run.fields.len(), &ident.to_string());
    let ident = ident.to_string();

    let mut idents = Vec::with_capacity(run.fields.len());
    let mut extracts = TokenStream::new();
    let mut at = 0usize;
    for (offset, field) in run.fields.iter().enumerate() {
        let f = fields.fields[start + offset];
        let field_ident = f.get_ident(start + offset, true);
        let internal = gen_internal_field_ident(&field_ident);
        let (ty, bytes, end) = (&field.ty, field.bytes, at + field.bytes);
        let trace_field_log = if cfg!(feature = "logging") {
            let field_ident_str = field_ident.to_string();
            quote! { ::#crate_::log::trace!("Reading: {}.{}", #ident, #field_ident_str); }
        } else {
            quote! {}
        };
        extracts.extend(quote! {
            #trace_field_log
            let #internal = {
                let __deku_bytes = <[u8; #bytes]>::try_from(&#buffer[#at..#end])
                    .expect("a byte run holds the bytes of each of its fields");
                if #endian.is_le() {
                    <#ty>::from_le_bytes(__deku_bytes)
                } else {
                    <#ty>::from_be_bytes(__deku_bytes)
                }
            };
            let #field_ident = &#internal;
        });
        idents.push(field_ident);
        at = end;
    }

    let read = quote! {
        let #endian: ::#crate_::ctx::Endian = #endian_value;
        let mut #buffer = [0u8; #total];
        __deku_reader.read_bytes_const_into::<#total>(&mut #buffer, ::#crate_::ctx::Order::default())?;
        #extracts
        #assertions
    };
    Ok((idents, read))
}

/// One read for the whole run, then shift and mask each field out of it. This is
/// what a hand-written parser does, and it replaces one `DekuReader` call plus one
/// bit-cursor update per field with one of each per run.
#[cfg(feature = "bits")]
fn emit_bit_run_read(
    input: &DekuData,
    fields: &Fields<&FieldData>,
    start: usize,
    run: &super::BitRun,
    ident: &TokenStream,
) -> Result<(Vec<TokenStream>, TokenStream), syn::Error> {
    let crate_ = super::get_crate_name();
    let refs: Vec<&BitRunField> = run.iter().collect();
    let total = sum_bits(&refs, &crate_);
    let run_ident = quote::format_ident!("__deku_bit_run_{}", start);
    let ident_str = ident.to_string();

    let mut idents = Vec::with_capacity(run.len());
    let mut internals = Vec::with_capacity(run.len());
    let mut extracts = TokenStream::new();
    // With a fallback, bind the fields after the `if const`.
    let fallback = needs_fallback(run);
    for (offset, field) in run.iter().enumerate() {
        let f = fields.fields[start + offset];
        let field_ident = f.get_ident(start + offset, true);
        let internal = gen_internal_field_ident(&field_ident);
        // How many bits of the run sit below this field.
        let shift = super::shift_of(&refs, offset, &crate_);
        let extract = super::extract_run_field(field, &quote! { #run_ident }, &shift, &crate_);
        let trace_field_log = if cfg!(feature = "logging") {
            let field_ident_str = field_ident.to_string();
            quote! { ::#crate_::log::trace!("Reading: {}.{}", #ident_str, #field_ident_str); }
        } else {
            quote! {}
        };
        extracts.extend(quote! {
            #trace_field_log
            let #internal = #extract;
        });
        if !fallback {
            extracts.extend(quote! { let #field_ident = &#internal; });
        }
        idents.push(field_ident);
        internals.push(internal);
    }

    // A run of whole bytes on a cursor at a byte boundary is plain bytes,
    // which one fixed-size read serves. The bit reader takes them one byte
    // at a time. A run with a `bit_field` has no literal width, so it stays
    // on the bit reader.
    let lit_total: Option<usize> = run.iter().map(BitRunField::lit_span).sum();
    let read_run = match lit_total {
        Some(total) if total % 8 == 0 => {
            let bytes = total / 8;
            let skip = 8 - bytes;
            quote! {
                if __deku_reader.leftover.is_none() {
                    let mut __deku_run_bytes = [0u8; 8];
                    __deku_reader.read_bytes_const_into::<#bytes>(
                        (&mut __deku_run_bytes[#skip..]).try_into().expect("a run of whole bytes"),
                        ::#crate_::ctx::Order::Msb0,
                    )?;
                    u64::from_be_bytes(__deku_run_bytes)
                } else {
                    __deku_reader.read_bits_uint_msb0(#total)?
                }
            }
        }
        // The same, where a `bit_field` makes the width a constant of a trait.
        // A generic type could make that width depend on a parameter, which
        // an array length cannot.
        //
        // The width is summed only for a run that passes its own checks, so
        // an absurd `BITS` does not overflow the sum.
        None if input.generics.params.is_empty() => {
            let batch = batch_condition(run, &crate_);
            quote! {{
            const __DEKU_RUN_BITS: usize = if #batch { #total } else { 0 };
            const __DEKU_RUN_BYTES: usize = if __DEKU_RUN_BITS % 8 == 0 { __DEKU_RUN_BITS / 8 } else { 0 };
            if __DEKU_RUN_BYTES != 0 && __deku_reader.leftover.is_none() {
                let mut __deku_run_bytes = [0u8; __DEKU_RUN_BYTES];
                __deku_reader.read_bytes_const_into(&mut __deku_run_bytes, ::#crate_::ctx::Order::Msb0)?;
                let mut __deku_run = 0u64;
                for __deku_byte in __deku_run_bytes {
                    __deku_run = (__deku_run << 8) | u64::from(__deku_byte);
                }
                __deku_run
            } else {
                __deku_reader.read_bits_uint_msb0(#total)?
            }
            }}
        }
        _ => quote! { __deku_reader.read_bits_uint_msb0(#total)? },
    };
    let ident_str_of_run = ident.to_string();
    let assertions = run_assertions(fields, start, run.len(), &ident_str_of_run);
    let batched = quote! {
        let #run_ident: u64 = #read_run;
        #extracts
    };

    if !fallback {
        return Ok((idents, quote! { #batched #assertions }));
    }

    // The field reads the run replaces, for when it turns out not to fit. Both
    // arms are type-checked, so a field still needs everything its own read
    // does, such as the context its type expects.
    let mut fallback = TokenStream::new();
    for offset in 0..run.len() {
        let (_, read) = emit_field_read(
            input,
            start + offset,
            fields.fields[start + offset],
            ident,
            false,
        )?;
        fallback.extend(read);
    }
    let batch = batch_condition(run, &crate_);
    let read = quote! {
        let (#(#internals,)*) = if const { #batch } {
            #batched
            #assertions
            (#(#internals,)*)
        } else {
            #fallback
            (#(#internals,)*)
        };
        #(let #idents = &#internals;)*
    };
    Ok((idents, read))
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
            let __deku_byte_offset = __deku_reader.bits_read / 8;
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
            let __deku_bit_offset = __deku_reader.bits_read;
        })
    } else {
        None
    };

    (bit_offset, byte_offset)
}

#[cfg(feature = "bits")]
fn emit_padding(bit_size: &TokenStream, bit_order: Option<&LitStr>) -> TokenStream {
    let crate_ = super::get_crate_name();
    if let Some(bit_order) = bit_order {
        let order = gen_bit_order_from_str(bit_order).unwrap();
        quote! {
            {
                use core::convert::TryFrom;
                let __deku_pad = usize::try_from(#bit_size).map_err(|e|
                    ::#crate_::deku_error!(::#crate_::DekuError::InvalidParam, "Invalid padding param, cannot convert ot usize", "{}", stringify!(#bit_size))
                )?;
                __deku_reader.skip_bits(__deku_pad, #order)?;
            }
        }
    } else {
        quote! {
            {
                use core::convert::TryFrom;
                let __deku_pad = usize::try_from(#bit_size).map_err(|e|
                    ::#crate_::deku_error!(::#crate_::DekuError::InvalidParam, "Invalid padding param, cannot convert to usize", "{}", stringify!(#bit_size))
                )?;
                __deku_reader.skip_bits(__deku_pad, ::#crate_::ctx::Order::default())?;
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

            while __deku_pad > 0 {
                let mut __deku_pad_source = [0u8; #pad];
                let __deku_pad_chunk = core::cmp::min(__deku_pad_source.len(), __deku_pad);
                __deku_reader.read_bytes(__deku_pad_chunk, &mut __deku_pad_source[..__deku_pad_chunk], ::#crate_::ctx::Order::default())?;
                __deku_pad -= __deku_pad_chunk;
            }
        }
    }
}

/// The `assert` and the `assert_eq` of the field `f` at `i`, as its read
/// checks them once the field is read.
fn read_assertions(
    f: &FieldData,
    i: usize,
    ident: &str,
) -> (Option<TokenStream>, Option<TokenStream>) {
    let field_ident = f.get_ident(i, true);
    let field_ident_str = field_ident.to_string();
    let internal_field_ident = gen_internal_field_ident(&field_ident);
    let field_assert = f.assert.as_ref().map(|v| {
        let return_error = assertion_failed(v, ident, &field_ident_str, None);
        quote! {
            if (!(#v)) {
                #return_error
            }
        }
    });
    let field_assert_eq = f.assert_eq.as_ref().map(|v| {
        let return_error = assertion_failed(v, ident, &field_ident_str, Some(&field_ident));
        quote! {
            if (!(#internal_field_ident == (#v))) {
                #return_error
            } else {
                // do nothing
            }
        }
    });
    (field_assert, field_assert_eq)
}

/// The assertions of the fields of a run that starts at `start`, checked
/// in field order once the whole run is read, as each field's own read
/// would have checked them.
pub(crate) fn run_assertions(
    fields: &Fields<&FieldData>,
    start: usize,
    len: usize,
    ident: &str,
) -> TokenStream {
    let mut checks = TokenStream::new();
    for offset in 0..len {
        let (assert, assert_eq) =
            read_assertions(fields.fields[start + offset], start + offset, ident);
        checks.extend(assert);
        checks.extend(assert_eq);
    }
    checks
}

fn emit_field_read(
    input: &DekuData,
    i: usize,
    f: &FieldData,
    ident: &TokenStream,
    pad_id: bool,
) -> Result<(TokenStream, TokenStream), syn::Error> {
    let crate_ = super::get_crate_name();
    let field_type = &f.ty;

    let field_endian = f.endian.as_ref().or(input.endian.as_ref());
    let field_bit_order = f.bit_order.as_ref().or(input.bit_order.as_ref());

    let field_reader = &f.reader;

    // fields to check usage of bit/byte offset
    let field_check_vars = [
        &f.count,
        #[cfg(feature = "bits")]
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

    let seek = if let Some(num) = &f.seek_from_current {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                if let Err(e) = __deku_reader.seek(SeekFrom::Current(i64::try_from(#num).unwrap())) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if let Some(num) = &f.seek_from_end {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                if let Err(e) = __deku_reader.seek(SeekFrom::End(i64::try_from(#num).unwrap())) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if let Some(num) = &f.seek_from_start {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                use ::#crate_::no_std_io::SeekFrom;
                if let Err(e) = __deku_reader.seek(SeekFrom::Start(u64::try_from(#num).unwrap())) {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else if f.seek_rewind {
        quote! {
            {
                use ::#crate_::no_std_io::Seek;
                if let Err(e) = __deku_reader.rewind() {
                    return Err(::#crate_::DekuError::Io(e.kind()));
                }
            }
        }
    } else {
        quote! {}
    };

    let (bit_offset, byte_offset) = emit_bit_byte_offsets(&field_check_vars);

    let field_map = f
        .map
        .as_ref()
        .map(|v| {
            quote! { (#v) }
        })
        .or_else(|| Some(quote! { core::result::Result::<_, ::#crate_::DekuError>::Ok }));

    let ident = ident.to_string();
    let field_ident = f.get_ident(i, true);
    let field_ident_str = field_ident.to_string();
    let internal_field_ident = gen_internal_field_ident(&field_ident);

    let (field_assert, field_assert_eq) = read_assertions(f, i, &ident);

    let trace_field_log = if cfg!(feature = "logging") {
        quote! {
            ::#crate_::log::trace!("Reading: {}.{}", #ident, #field_ident_str);
        }
    } else {
        quote! {}
    };

    let magic_read = if let Some(magic) = &f.magic {
        emit_magic_read_lit(&crate_, magic)
    } else {
        quote! {}
    };

    let field_read_func = if field_reader.is_some() {
        quote! { #field_reader? }
    } else {
        let read_args = gen_field_args(
            field_endian,
            #[cfg(feature = "bits")]
            f.bits.as_ref(),
            #[cfg(not(feature = "bits"))]
            None,
            f.bytes.as_ref(),
            f.ctx.as_ref(),
            field_bit_order,
        )?;

        // The __deku_reader limiting options are special, we need to generate `(limit, (other, ..))` for them.
        // These have a problem where when it isn't a copy type, the field will be moved.
        // e.g. struct FooBar {
        //   a: Baz // a type implement `Into<usize>` but not `Copy`.
        //   #[deku(count = "a") <-- Oops, use of moved value: `a`
        //   b: Vec<_>
        // }

        let type_as_deku_read = if f.map.is_some() {
            // with map, field_type cannot be used as the
            // resulting type is within the function.
            quote!(::#crate_::DekuReader)
        } else {
            // use type directly
            quote!(<#field_type as ::#crate_::DekuReader<'_, _>>)
        };

        if pad_id {
            if f.any_field_set() {
                // TODO: This would be nice to point to the field
                return Err(syn::Error::new(
                    input.ident.span(),
                    "DekuRead: id_pat id storage cannot have attributes",
                ));
            }
            quote! {
                __deku_variant_id;
            }
        } else if let Some(field_count) = &f.count {
            use syn::{GenericArgument, PathArguments, Type};
            let mut is_vec_u8 = false;
            if let Type::Path(type_path) = &f.ty
                && type_path.path.segments.len() == 1
                && type_path.path.segments[0].ident == "Vec"
                && let PathArguments::AngleBracketed(generic_args) =
                    &type_path.path.segments[0].arguments
                && generic_args.args.len() == 1
                && let GenericArgument::Type(Type::Path(arg_path)) = &generic_args.args[0]
            {
                is_vec_u8 = arg_path.path.is_ident("u8");
            }
            if is_vec_u8 {
                quote! {
                    {
                        use core::borrow::Borrow;
                        #type_as_deku_read::from_reader_with_ctx
                        (
                            __deku_reader,
                            ::#crate_::ctx::ReadExact(usize::try_from(*((#field_count).borrow()))?)
                        )?
                    }
                }
            } else {
                quote! {
                    {
                        use core::borrow::Borrow;
                        #type_as_deku_read::from_reader_with_ctx
                        (
                            __deku_reader,
                            (::#crate_::ctx::Limit::new_count(usize::try_from(*((#field_count).borrow()))?), (#read_args))
                        )?
                    }
                }
            }
        } else if let Some(field_bytes) = &f.bytes_read {
            quote! {
                {
                    use core::borrow::Borrow;
                    #type_as_deku_read::from_reader_with_ctx
                    (
                        __deku_reader,
                        (::#crate_::ctx::Limit::new_byte_size(::#crate_::ctx::ByteSize(usize::try_from(*((#field_bytes).borrow()))?)), (#read_args))
                    )?
                }
            }
        } else if let Some(field_until) = &f.until {
            // We wrap the input into another closure here to enforce that it is actually a callable
            // Otherwise, an incorrectly passed-in integer could unexpectedly convert into a `Count` limit
            quote! {
                #type_as_deku_read::from_reader_with_ctx
                (
                    __deku_reader,
                    (::#crate_::ctx::Limit::new_until(#field_until), (#read_args))
                )?
            }
        } else if f.read_all {
            quote! {
                {
                    use core::borrow::Borrow;
                    #type_as_deku_read::from_reader_with_ctx
                    (
                        __deku_reader,
                        (::#crate_::ctx::Limit::end(), (#read_args))
                    )?
                }
            }
        } else {
            let mut ret = quote! {};

            #[cfg(feature = "bits")]
            if let Some(field_bits) = &f.bits_read {
                ret.extend(quote! {
                    {
                        use core::borrow::Borrow;
                        #type_as_deku_read::from_reader_with_ctx
                        (
                            __deku_reader,
                            (::#crate_::ctx::Limit::new_bit_size(::#crate_::ctx::BitSize(usize::try_from(*((#field_bits).borrow()))?)), (#read_args))
                        )?
                    }
                })
            }
            // One `read_exact` for a plain byte array, in place of one read per
            // element through the generic `[T; N]` impl.
            if ret.is_empty()
                && let Some(n) = byte_array_len(input, f)
            {
                ret.extend(quote! {
                    {
                        let mut __deku_bytes = [0u8; #n];
                        __deku_reader.read_bytes_const_into::<#n>(
                            &mut __deku_bytes,
                            ::#crate_::ctx::Order::Msb0,
                        )?;
                        __deku_bytes
                    }
                })
            }

            if ret.is_empty() {
                ret.extend(quote! {
                    #type_as_deku_read::from_reader_with_ctx
                    (
                        __deku_reader,
                        (#read_args)
                    )?
                })
            }

            ret
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

    let field_read_normal = quote! {
        let __deku_value = #field_read_func;
        let __deku_value: #field_type = #field_map(__deku_value)?;
        __deku_value
    };

    let field_default = &f.default;

    let field_read_tokens = match (&f.skip, &f.cond) {
        (Some(crate::SkipMode::All), Some(field_cond))
        | (Some(crate::SkipMode::Read), Some(field_cond)) => {
            // #[deku(skip, cond = "...")] or #[deku(skip(read), cond = "...")] ==> `skip` if `cond`
            quote! {
                if (#field_cond) {
                    #field_default
                } else {
                    #field_read_normal
                }
            }
        }
        (Some(crate::SkipMode::All), None) | (Some(crate::SkipMode::Read), None) => {
            // #[deku(skip)] or #[deku(skip(read))] ==> `skip` reading
            quote! {
                #field_default
            }
        }
        (Some(crate::SkipMode::Write), _) => {
            // #[deku(skip(write))] ==> read normally
            quote! {
                #field_read_normal
            }
        }
        (None, Some(field_cond)) => {
            // #[deku(cond = "...")] ==> read if `cond`
            quote! {
                if (#field_cond) {
                    #field_read_normal
                } else {
                    #field_default
                }
            }
        }
        (None, None) => {
            quote! {
                #field_read_normal
            }
        }
    };

    let field_read = quote! {
        #seek
        #magic_read
        #pad_bits_before

        #bit_offset
        #byte_offset

        #trace_field_log
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

/// emit `from_reader()` and `from_bytes()` for struct/enum
pub fn emit_container_read(
    imp: &syn::ImplGenerics,
    lifetime: &TokenStream,
    ident: &TokenStream,
    wher: Option<&syn::WhereClause>,
    from_reader_body: TokenStream,
    from_bytes_body: TokenStream,
) -> TokenStream {
    let crate_ = super::get_crate_name();
    quote! {
        #[automatically_derived]
        impl #imp ::#crate_::DekuContainerRead<#lifetime> for #ident #wher {
            #[allow(non_snake_case)]
            #[inline]
            fn from_reader<'a, R: ::#crate_::no_std_io::Read + ::#crate_::no_std_io::Seek>(__deku_input: (&'a mut R, usize)) -> core::result::Result<(usize, Self), ::#crate_::DekuError> {
                #from_reader_body
            }

            #[allow(non_snake_case)]
            #[inline]
            fn from_bytes(__deku_input: (&#lifetime [u8], usize)) -> core::result::Result<((&#lifetime [u8], usize), Self), ::#crate_::DekuError> {
                #from_bytes_body
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
    let crate_ = super::get_crate_name();
    quote! {
        #[automatically_derived]
        impl #imp core::convert::TryFrom<&#lifetime [u8]> for #ident #wher {
            type Error = ::#crate_::DekuError;

            #[inline]
            fn try_from(input: &#lifetime [u8]) -> core::result::Result<Self, Self::Error> {
                let total_len = input.len();
                let mut cursor = ::#crate_::no_std_io::Cursor::new(input);
                let (bits_read, res) = <Self as ::#crate_::DekuContainerRead>::from_reader((&mut cursor, 0))?;
                let bytes_read = bits_read / 8;
                if bytes_read < total_len {
                    return Err(::#crate_::deku_error!(::#crate_::DekuError::Parse, "Too much data", "Read {} but total length was {}", {bits_read / 8}, total_len));
                }
                // Possible Seek beyond end
                if bytes_read > total_len {
                    return Err(::#crate_::DekuError::Incomplete(::#crate_::error::NeedSize::new(bits_read - { total_len * 8 })));
                }
                Ok(res)
            }
        }
    }
}

/// The read of an enum whose id and every variant are bits of a run: one
/// read of the id with as many bits after it as the narrowest variant
/// holds, then, for a wider variant, one read of the bits it has left.
/// The fields come out of those bits as out of a struct's run, so the enum
/// costs one or two reads where its id and its fields cost one each.
///
/// The constants of the fused read come apart from the read itself, so
/// that the caller can choose between it and the unbatched reads with
/// `__DEKU_FUSED`.
///
/// `None` where the enum does not qualify: an id that is not a literal
/// width in big-endian `Msb0`, an id from context, a `magic`, a variant
/// with a `reader`, a `default` or a field that no run can serve, or no
/// bits after the id in some variant. Those keep the reads of the id and
/// fields one at a time.
///
/// As for the run of a struct, input too short for the first read is
/// `Incomplete` even where the id alone would be unassigned.
#[cfg(feature = "bits")]
pub(crate) fn emit_fused_enum_read(input: &DekuData) -> Option<FusedEnumRead> {
    let crate_ = super::get_crate_name();
    let id_bits = super::const_id_bits(input)?;
    if input.id.is_some() || input.magic.is_some() || !input.generics.params.is_empty() {
        return None;
    }
    let id_type = input.id_type.as_ref()?;
    let Data::Enum(variants) = &input.data else {
        return None;
    };
    if variants.is_empty() {
        return None;
    }

    /// One variant: its id pattern, the run of its fields, and whether its
    /// first field stores the id.
    struct Plan<'a> {
        variant: &'a crate::VariantData,
        pattern: TokenStream,
        stores_id: bool,
        run: Vec<BitRunField>,
    }

    let mut plans = Vec::with_capacity(variants.len());
    for variant in variants {
        if variant.reader.is_some() || variant.default.unwrap_or(false) {
            return None;
        }
        let (pattern, stores_id) = match (&variant.id, &variant.id_pat) {
            (Some(Id::Int(id)), _) => (id.to_token_stream(), false),
            // As the unbatched match does, a token id matches by reference.
            (Some(Id::TokenStream(id)), _) => (quote! { &#id }, false),
            (Some(_), _) => return None,
            // Without an `id` of the enum, the first field of an `id_pat`
            // variant holds the id, as its unbatched read stores it.
            (None, Some(pat)) => (pat.clone(), !variant.fields.is_empty()),
            (None, None) => return None,
        };
        let fields = &variant.fields.fields;
        let run_fields = if stores_id {
            if fields[0].any_field_set() {
                return None;
            }
            &fields[1..]
        } else {
            &fields[..]
        };
        let run = run_fields
            .iter()
            .map(|f| run_field(input, f))
            .collect::<Option<Vec<_>>>()?;
        plans.push(Plan {
            variant,
            pattern,
            stores_id,
            run,
        });
    }

    // The bits of each variant after the id, as a constant expression, and
    // the checks that make each a valid run.
    let widths: Vec<TokenStream> = plans
        .iter()
        .map(|plan| {
            if plan.run.is_empty() {
                quote! { 0usize }
            } else {
                let refs: Vec<&BitRunField> = plan.run.iter().collect();
                sum_bits(&refs, &crate_)
            }
        })
        .collect();
    let lit_widths: Option<Vec<usize>> = plans
        .iter()
        .map(|plan| plan.run.iter().map(BitRunField::lit_span).sum())
        .collect();
    // A variant with no bits after the id leaves nothing to read with it.
    if let Some(lit) = &lit_widths
        && (lit.contains(&0) || lit.iter().any(|w| id_bits + w > 64))
    {
        return None;
    }
    let trait_checks: Vec<TokenStream> = plans
        .iter()
        .flat_map(|plan| plan.run.iter())
        .filter(|f| f.lit_bits().is_none())
        .map(|f| {
            let bits = f.bits(&crate_);
            quote! { (#bits != 0 && #bits <= 64) }
        })
        .collect();
    // Each width is bounded before any sum, so that an absurd `BITS` falls
    // back where it would otherwise overflow.
    let fits = quote! { #(#trait_checks &&)* true };
    let width_checks = widths
        .iter()
        .map(|w| quote! { (#w >= 1 && #w + #id_bits <= 64) });
    let fused = quote! { #fits && #(#width_checks)&&* };
    let narrowest = widths.iter().skip(1).fold(widths[0].clone(), |acc, w| {
        quote! { { let __deku_a = #acc; let __deku_b = #w; if __deku_b < __deku_a { __deku_b } else { __deku_a } } }
    });

    let mut arms = Vec::with_capacity(plans.len());
    for (plan, width) in plans.iter().zip(&widths) {
        let fields = plan.variant.fields.as_ref();
        let named = fields.style.is_struct();
        let refs: Vec<&BitRunField> = plan.run.iter().collect();
        let raw = quote! { __deku_raw };
        let mut binds = TokenStream::new();
        let mut idents = Vec::with_capacity(fields.len());
        for (index, f) in fields.fields.iter().enumerate() {
            let field_ident = f.get_ident(index, true);
            let internal = gen_internal_field_ident(&field_ident);
            if plan.stores_id && index == 0 {
                binds.extend(quote! { let #internal = __deku_variant_id; });
            } else {
                let offset = index - usize::from(plan.stores_id);
                let shift = super::shift_of(&refs, offset, &crate_);
                let extract = super::extract_run_field(&plan.run[offset], &raw, &shift, &crate_);
                binds.extend(quote! { let #internal = #extract; });
            }
            if !f.temp {
                idents.push(field_ident);
            }
        }
        let variant_ident = &plan.variant.ident;
        let init = super::gen_enum_init(
            named,
            quote! { Self::#variant_ident },
            gen_internal_field_idents(named, idents.iter()),
        );
        let rest = read_bits(&quote! { __DEKU_REST_BITS }, &crate_);
        let assertions = run_assertions(
            &fields,
            usize::from(plan.stores_id),
            plan.run.len(),
            &input.ident.to_string(),
        );
        let pattern = &plan.pattern;
        arms.push(quote! {
            #pattern => {
                const __DEKU_REST_BITS: usize =
                    if __DEKU_FUSED { #width - __DEKU_HEAD_PAYLOAD_BITS } else { 0 };
                let __deku_raw: u64 = if __DEKU_REST_BITS == 0 {
                    __deku_head
                } else {
                    (__deku_head << __DEKU_REST_BITS) | #rest
                };
                #binds
                #assertions
                #init
            }
        });
    }
    let has_catch_all = plans.iter().any(|plan| plan.pattern.to_string() == "_");
    let no_variant = super::no_variant_error(&crate_, &input.ident.to_string());
    let catch_all = (!has_catch_all).then(|| quote! { _ => return Err(#no_variant), });
    let head = read_bits(&quote! { __DEKU_HEAD_BITS }, &crate_);
    let id_mask = super::low_mask(id_bits);

    Some(FusedEnumRead {
        consts: quote! {
            const __DEKU_FUSED: bool = #fused;
            const __DEKU_HEAD_PAYLOAD_BITS: usize = if __DEKU_FUSED { #narrowest } else { 0 };
            const __DEKU_HEAD_BITS: usize = #id_bits + __DEKU_HEAD_PAYLOAD_BITS;
        },
        read: quote! {{
            let __deku_head: u64 = #head;
            let __deku_variant_id = ((__deku_head >> __DEKU_HEAD_PAYLOAD_BITS) & #id_mask) as #id_type;
            let __deku_head = __deku_head & ((1u64 << __DEKU_HEAD_PAYLOAD_BITS) - 1);
            match &__deku_variant_id {
                #(#arms)*
                #catch_all
            }
        }},
    })
}

/// The parts of [`emit_fused_enum_read`].
#[cfg(feature = "bits")]
pub(crate) struct FusedEnumRead {
    /// Constant items, `__DEKU_FUSED` among them, for the scope of the read.
    consts: TokenStream,
    /// An expression of the enum's value, for where `__DEKU_FUSED` holds.
    read: TokenStream,
}

/// `bits` bits, a constant, as a `u64`: one fixed-size read of whole bytes
/// on a byte boundary, and the bit reader otherwise.
#[cfg(feature = "bits")]
fn read_bits(bits: &TokenStream, crate_: &Ident) -> TokenStream {
    quote! {{
        const __DEKU_READ_BYTES: usize =
            if #bits % 8 == 0 && #bits <= 64 { #bits / 8 } else { 0 };
        if __DEKU_READ_BYTES != 0 && __deku_reader.leftover.is_none() {
            let mut __deku_bytes = [0u8; __DEKU_READ_BYTES];
            __deku_reader.read_bytes_const_into(&mut __deku_bytes, ::#crate_::ctx::Order::Msb0)?;
            let mut __deku_bits = 0u64;
            for __deku_byte in __deku_bytes {
                __deku_bits = (__deku_bits << 8) | u64::from(__deku_byte);
            }
            __deku_bits
        } else {
            __deku_reader.read_bits_uint_msb0(#bits)?
        }
    }}
}
