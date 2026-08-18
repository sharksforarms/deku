use std::convert::TryFrom;

use darling::ast::{Data, Fields};
use darling::ToTokens;
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
use crate::{from_token, DekuData, DekuDataEnum, DekuDataStruct, FieldData, Id};

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
            let __deku_reader = &mut deku::reader::Reader::new(__deku_input.0);
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
            let mut __deku_reader = &mut deku::reader::Reader::new(&mut __deku_cursor);
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
                return Err(deku::DekuError::Incomplete(deku::prelude::NeedSize::new(8 * (idx - __deku_input.0.len()))));
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
        variant_matches.push(quote! {
            _ => {
                return Err(::#crate_::deku_error!(::#crate_::DekuError::Parse, "Could not match enum variant", "ID {:?} not found on {}", __deku_variant_id, #ident_as_string));
            }
        });
    }

    // if default
    if !has_default_match {
        if let Some(variant_read_func) = default_reader {
            variant_matches.push(quote! {
                _ => {
                    #variant_read_func
                }
            });
        }
    }

    let variant_id_read = if id.is_some() {
        quote! {
            let __deku_variant_id = (#id);
        }
    } else if id_type.is_some() {
        quote! {
            let __deku_variant_id = <#id_type>::from_reader_with_ctx(__deku_reader,  (#id_args))?;
        }
    } else {
        // either `id` or `id_type` needs to be specified
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
        let from_reader_body = quote! {
            use core::convert::TryFrom;
            use ::#crate_::DekuReader as _;
            let __deku_reader = &mut deku::reader::Reader::new(__deku_input.0);
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
            let mut __deku_reader = &mut deku::reader::Reader::new(&mut __deku_cursor);
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
                return Err(deku::DekuError::Incomplete(deku::prelude::NeedSize::new(8 * (idx - __deku_input.0.len()))));
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
    let runs = plan_bit_runs(input, fields, use_id);

    let mut i = 0;
    while i < fields.len() {
        #[cfg(feature = "bits")]
        if let Some(run) = runs.get(&i) {
            let (idents, read) = emit_bit_run_read(fields, i, run);
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

        let f = fields.fields[i];
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

/// One field of a contiguous big-endian `Msb0` bit-field run.
#[cfg(feature = "bits")]
pub(crate) struct BitRunField {
    pub(crate) bits: usize,
    pub(crate) ty: syn::Type,
    /// Whether the field's own write would have gone through the `Order`-carrying
    /// impl, which words its overflow error differently. Selects the wording the
    /// batched write reports.
    pub(crate) ordered: bool,
    /// Whether a written value can actually exceed `bits`. False where the field
    /// fills its own type, or is a `bool`, since neither can overflow: the check
    /// would always pass, so the run does not emit one.
    pub(crate) can_overflow: bool,
}

/// Widths of a run of adjacent fields that one read can serve.
#[cfg(feature = "bits")]
pub(crate) type BitRun = Vec<BitRunField>;

/// A field that a run read can serve: `bits = N` with a literal `N`, on a plain
/// unsigned primitive, explicitly big-endian, `Msb0`, and carrying no attribute
/// a batched read cannot reproduce. Anything else keeps its own read.
#[cfg(feature = "bits")]
pub(crate) fn run_field(input: &DekuData, f: &FieldData) -> Option<BitRunField> {
    if f.any_field_set_incompatible_with_bit_run() {
        return None;
    }

    // Big-endian must be explicit: with no attribute the context endian is the
    // target's, which is little on x86.
    let endian = f.endian.as_ref().or(input.endian.as_ref())?;
    if endian.value() != "big" {
        return None;
    }

    // Only `Msb0` batches, and it is the default, so absent is fine and "lsb" is not.
    let explicit_order = f.bit_order.as_ref().or(input.bit_order.as_ref());
    if let Some(order) = explicit_order {
        if order.value() != "msb" {
            return None;
        }
    }
    // Same check, two spellings: `DekuWriter<(Endian, BitSize, Order)>` drops the
    // second "bit" that `DekuWriter<(Endian, BitSize)>` includes. Record which one
    // this field would have produced so batching does not change the message.
    let ordered = explicit_order.is_some();

    let width = match &f.ty {
        syn::Type::Path(p) if p.qself.is_none() => match p.path.get_ident()?.to_string().as_str() {
            "u8" => u8::BITS as usize,
            "u16" => u16::BITS as usize,
            "u32" => u32::BITS as usize,
            "u64" => u64::BITS as usize,
            // `impls::bool` reads a bool by delegating to `u8` with the same ctx,
            // so it occupies a byte unless `bits` narrows it. Flags in a packed
            // header are usually `bits = 1`.
            "bool" => u8::BITS as usize,
            _ => return None,
        },
        _ => return None,
    };

    let bits = match f.bits.as_ref() {
        Some(crate::Num::LitInt(lit)) => lit.base10_parse::<usize>().ok()?,
        Some(crate::Num::TokenStream(_)) => return None,
        // A plain big-endian integer field is exactly `bits = width`: when the
        // cursor is byte-aligned it is a big-endian byte read, and when it is not,
        // deku already routes it through `read_bits_into` for the same `width`
        // bits, most-significant first.
        None => width,
    };
    if bits == 0 || bits > width {
        return None;
    }

    // A value cast from a type `bits` wide cannot need more than `bits` bits, and a
    // bool is 0 or 1, so in both cases the per-field check could never have fired.
    let can_overflow = bits < width && !is_bool(&f.ty);

    Some(BitRunField {
        bits,
        ty: f.ty.clone(),
        ordered,
        can_overflow,
    })
}

/// Whether the field is a plain `bool`, which a run must compare rather than cast.
#[cfg(feature = "bits")]
fn is_bool(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.qself.is_none() && p.path.is_ident("bool"))
}

/// Groups adjacent run-eligible fields, keyed by the index the run starts at.
///
/// A run is capped at 64 bits, the width the reader returns, and must hold at
/// least two fields to be worth a batch.
#[cfg(feature = "bits")]
pub(crate) fn plan_bit_runs(
    input: &DekuData,
    fields: &Fields<&FieldData>,
    use_id: bool,
) -> std::collections::HashMap<usize, BitRun> {
    let mut runs = std::collections::HashMap::new();
    let mut i = 0;
    while i < fields.len() {
        // The first field can be the enum id storage, which is not a read at all.
        if i == 0 && use_id {
            i = 1;
            continue;
        }
        let mut run: BitRun = Vec::new();
        let mut total = 0usize;
        let mut j = i;
        while j < fields.len() {
            let Some(field) = run_field(input, fields.fields[j]) else {
                break;
            };
            if total + field.bits > u64::BITS as usize {
                break;
            }
            total += field.bits;
            run.push(field);
            j += 1;
        }
        if run.len() >= 2 {
            let len = run.len();
            runs.insert(i, run);
            i += len;
        } else {
            i += 1;
        }
    }
    runs
}

/// One read for the whole run, then shift and mask each field out of it. This is
/// what a hand-written parser does, and it replaces one `DekuReader` call plus one
/// bit-cursor update per field with one of each per run.
#[cfg(feature = "bits")]
fn emit_bit_run_read(
    fields: &Fields<&FieldData>,
    start: usize,
    run: &BitRun,
) -> (Vec<TokenStream>, TokenStream) {
    let crate_ = super::get_crate_name();
    let total: usize = run.iter().map(|f| f.bits).sum();
    let run_ident = quote::format_ident!("__deku_bit_run_{}", start);

    let mut idents = Vec::with_capacity(run.len());
    let mut extracts = TokenStream::new();
    let mut consumed = 0usize;
    for (offset, field) in run.iter().enumerate() {
        let f = fields.fields[start + offset];
        let field_ident = f.get_ident(start + offset, true);
        let internal = gen_internal_field_ident(&field_ident);
        let shift = total - consumed - field.bits;
        // A run holds at least two fields totalling at most 64 bits, so no single
        // field in one is 64 bits wide and the shift below cannot overflow.
        debug_assert!(field.bits < u64::BITS as usize);
        let mask: u64 = (1u64 << field.bits) - 1;
        let ty = &field.ty;
        // `as` cannot produce a bool, so a bool field is compared rather than cast.
        let extract = if is_bool(ty) {
            if field.bits == 1 {
                // One bit is either 0 or 1, so there is no invalid value to reject.
                quote! { ((#run_ident >> #shift) & 1) != 0 }
            } else {
                // A wider bool rejects anything but 0 and 1, with the same error
                // `impls::bool` returns.
                quote! {
                    match (#run_ident >> #shift) & #mask {
                        0 => false,
                        1 => true,
                        __deku_bool => return Err(::#crate_::deku_error!(
                            ::#crate_::DekuError::Parse,
                            "cannot parse bool value",
                            "{}",
                            __deku_bool as u8
                        )),
                    }
                }
            }
        } else {
            quote! { ((#run_ident >> #shift) & #mask) as #ty }
        };
        extracts.extend(quote! {
            let #internal = #extract;
            let #field_ident = &#internal;
        });
        idents.push(field_ident);
        consumed += field.bits;
    }

    let read = quote! {
        let #run_ident: u64 = __deku_reader.read_bits_uint_msb0(#total)?;
        #extracts
    };
    (idents, read)
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

    let field_assert = f.assert.as_ref().map(|v| {
        let return_error = assertion_failed(v, &ident, &field_ident_str, None);
        quote! {
            if (!(#v)) {
                #return_error
            }
        }
    });

    let field_assert_eq = f.assert_eq.as_ref().map(|v| {
        let return_error = assertion_failed(v, &ident, &field_ident_str, Some(&field_ident));
        quote! {
            if (!(#internal_field_ident == (#v))) {
                #return_error
            } else {
                // do nothing
            }
        }
    });

    let trace_field_log = if cfg!(feature = "logging") {
        quote! {
            log::trace!("Reading: {}.{}", #ident, #field_ident_str);
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
            if let Type::Path(type_path) = &f.ty {
                if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
                    if let PathArguments::AngleBracketed(ref generic_args) =
                        type_path.path.segments[0].arguments
                    {
                        if generic_args.args.len() == 1 {
                            if let GenericArgument::Type(Type::Path(ref arg_path)) =
                                generic_args.args[0]
                            {
                                is_vec_u8 = arg_path.path.is_ident("u8");
                            }
                        }
                    }
                }
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

#[cfg(test)]
#[cfg(feature = "bits")]
mod tests {
    use rstest::rstest;

    use super::*;

    /// Sorts a planner result into `(index of the first field, widths)` pairs.
    fn sorted(runs: std::collections::HashMap<usize, BitRun>) -> Vec<(usize, Vec<usize>)> {
        let mut runs: Vec<_> = runs
            .into_iter()
            .map(|(start, run)| (start, run.iter().map(|f| f.bits).collect::<Vec<_>>()))
            .collect();
        runs.sort_by_key(|(start, _)| *start);
        runs
    }

    /// Every run the planner forms over a struct.
    fn plan(src: &str) -> Vec<(usize, Vec<usize>)> {
        plan_with_id(src, false)
    }

    /// As `plan`, with control over `use_id`, which tells the planner the first
    /// field is an enum's id storage rather than something to read.
    fn plan_with_id(src: &str, use_id: bool) -> Vec<(usize, Vec<usize>)> {
        let data = DekuData::from_input(src.parse().unwrap()).expect("input should parse");
        let fields = data
            .data
            .as_ref()
            .take_struct()
            .expect("test input should be a struct");

        sorted(plan_bit_runs(&data, &fields, use_id))
    }

    /// Every run the planner forms over one variant of an enum.
    fn plan_variant(src: &str, variant: usize, use_id: bool) -> Vec<(usize, Vec<usize>)> {
        let data = DekuData::from_input(src.parse().unwrap()).expect("input should parse");
        let variants = data
            .data
            .as_ref()
            .take_enum()
            .expect("test input should be an enum");
        let fields = variants[variant].fields.as_ref();

        sorted(plan_bit_runs(&data, &fields, use_id))
    }

    /// A struct of big-endian `u8` fields, one per `bits` width given.
    fn be_struct(widths: &[usize]) -> String {
        let fields: String = widths
            .iter()
            .enumerate()
            .map(|(i, w)| format!("#[deku(bits = {w})] f{i}: u8,"))
            .collect();
        format!(r#"#[deku(endian = "big")] struct Test {{ {fields} }}"#)
    }

    #[test]
    fn adjacent_fields_share_one_read() {
        assert_eq!(plan(&be_struct(&[2, 3, 3])), vec![(0, vec![2, 3, 3])]);
    }

    #[test]
    fn a_lone_field_is_not_a_run() {
        // One field would cost the same read either way, so batching buys nothing.
        assert_eq!(plan(&be_struct(&[5])), vec![]);
    }

    #[test]
    fn plain_fields_without_bits_are_their_full_width() {
        let src = r#"#[deku(endian = "big")] struct Test { a: u8, b: u16, c: u32 }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 16, 32])]);
    }

    #[test]
    fn a_run_is_capped_at_64_bits_and_the_next_one_starts_there() {
        // 32 + 32 fills a run exactly; the third field opens a second run, which
        // then needs a partner of its own to be worth forming.
        let src = r#"#[deku(endian = "big")] struct Test { a: u32, b: u32, c: u32, d: u32 }"#;
        assert_eq!(plan(src), vec![(0, vec![32, 32]), (2, vec![32, 32])]);

        // A field that does not fit closes the run rather than overflowing it.
        let src = r#"#[deku(endian = "big")] struct Test { a: u32, b: u16, c: u32 }"#;
        assert_eq!(plan(src), vec![(0, vec![32, 16])]);
    }

    #[test]
    fn an_ineligible_field_splits_a_run_in_two() {
        let src = r#"
        #[deku(endian = "big")]
        struct Test {
            #[deku(bits = 2)] a: u8,
            #[deku(bits = 2)] b: u8,
            #[deku(endian = "little")] c: u16,
            #[deku(bits = 2)] d: u8,
            #[deku(bits = 2)] e: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![2, 2]), (3, vec![2, 2])]);
    }

    #[test]
    fn endianness_must_be_explicitly_big() {
        // Absent means the target's endianness, which is little on x86, so the
        // planner must not assume it.
        assert_eq!(plan(r#"struct Test { a: u8, b: u8 }"#), vec![]);
        assert_eq!(
            plan(r#"#[deku(endian = "little")] struct Test { a: u8, b: u8 }"#),
            vec![]
        );
        // A field-level attribute qualifies a field inside a little-endian struct.
        let src = r#"#[deku(endian = "little")] struct Test {
            #[deku(endian = "big")] a: u8,
            #[deku(endian = "big")] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 8])]);
    }

    #[test]
    fn bit_order_must_be_msb() {
        // `Msb0` is the default, so absent qualifies, and so does spelling it out.
        let src = r#"#[deku(endian = "big")] struct Test { a: u8, b: u8 }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 8])]);

        let src = r#"#[deku(endian = "big", bit_order = "msb")] struct Test { a: u8, b: u8 }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 8])]);

        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bit_order = "msb")] a: u8,
            b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 8])]);

        // "lsb" does not.
        let src = r#"#[deku(endian = "big", bit_order = "lsb")] struct Test { a: u8, b: u8 }"#;
        assert_eq!(plan(src), vec![]);

        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bit_order = "lsb")] a: u8,
            b: u8,
        }"#;
        assert_eq!(plan(src), vec![]);
    }

    #[test]
    fn an_explicit_bit_order_selects_the_other_overflow_wording() {
        // Both fields batch, but they reach different write impls, which word the
        // overflow error differently, so each must call the matching check.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4, bit_order = "msb")] ordered: u8,
            #[deku(bits = 4)] plain: u8,
        }"#;
        let data = DekuData::from_input(src.parse().unwrap()).unwrap();
        let fields = data.data.as_ref().take_struct().unwrap();
        let runs = plan_bit_runs(&data, &fields, false);
        let run = runs.get(&0).expect("both fields should batch");
        assert_eq!(
            run.iter().map(|f| f.ordered).collect::<Vec<_>>(),
            vec![true, false]
        );

        let emitted = emit_deku_read(&data).unwrap().to_string();
        assert_eq!(emitted.matches("read_bits_uint_msb0").count(), 1);
    }

    #[test]
    fn a_bool_joins_a_run() {
        // Flags in a packed header are `bits = 1` bools, and excluding them splits
        // a run wherever a flag sits.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 2)] a: u8,
            #[deku(bits = 1)] flag: bool,
            #[deku(bits = 5)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![2, 1, 5])]);

        // Without `bits` a bool is a byte, as `impls::bool` reads it.
        let src = r#"#[deku(endian = "big")] struct Test { flag: bool, b: u8 }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 8])]);
    }

    #[test]
    fn the_rtp_header_batches_into_one_read() {
        // RFC 3550 fixed header: 9 fields, three of them flags. With bools excluded
        // this planned as a single run of three, leaving six standalone reads.
        let src = r#"#[deku(endian = "big")] struct Rtp {
            #[deku(bits = 2)] version: u8,
            #[deku(bits = 1)] padding: bool,
            #[deku(bits = 1)] extension: bool,
            #[deku(bits = 4)] csrc_count: u8,
            #[deku(bits = 1)] marker: bool,
            #[deku(bits = 7)] payload_type: u8,
            sequence_number: u16,
            timestamp: u32,
            ssrc: u32,
        }"#;
        // The first eight fields sum to exactly 64 bits; `ssrc` cannot fit and is
        // left alone, so the header costs two reads instead of seven.
        assert_eq!(plan(src), vec![(0, vec![2, 1, 1, 4, 1, 7, 16, 32])]);
    }

    #[test]
    fn only_unsigned_primitives_and_bool_qualify() {
        for ty in ["i8", "i16", "f32", "MyEnum", "Vec<u8>", "[u8; 2]"] {
            let src = format!(r#"#[deku(endian = "big")] struct Test {{ a: {ty}, b: {ty} }}"#);
            assert_eq!(plan(&src), vec![], "{ty} must not form a run");
        }
    }

    #[test]
    fn bits_must_be_a_literal_and_fit_the_type() {
        // A non-literal width is not known at expansion time.
        let src = r#"#[deku(endian = "big", ctx = "n: usize")] struct Test {
            #[deku(bits = "n")] a: u8,
            #[deku(bits = "n")] b: u8,
        }"#;
        assert_eq!(plan(src), vec![]);

        // Wider than its container.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 9)] a: u8,
            #[deku(bits = 2)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![]);
    }

    /// The deny list in `run_field` is the part most likely to rot, so pin every
    /// attribute that has to keep a field out of a run. Each case is two fields
    /// that would otherwise batch, with the attribute on the first.
    #[rstest]
    #[case::bytes("bytes = 1")]
    #[case::pad_bits_before("pad_bits_before = \"1\"")]
    #[case::pad_bytes_before("pad_bytes_before = \"1\"")]
    #[case::pad_bits_after("pad_bits_after = \"1\"")]
    #[case::pad_bytes_after("pad_bytes_after = \"1\"")]
    #[case::cond("cond = \"true\"")]
    #[case::assert("assert = \"true\"")]
    #[case::assert_eq("assert_eq = \"0\"")]
    #[case::map("map = \"|v: u8| -> Result<_, DekuError> { Ok(v) }\"")]
    #[case::reader("reader = \"read_it()\"")]
    #[case::writer("writer = \"write_it()\"")]
    #[case::skip_with_default("skip, default = \"0\"")]
    #[case::temp("temp")]
    #[case::seek_rewind("seek_rewind")]
    #[case::seek_from_current("seek_from_current = \"1\"")]
    #[case::seek_from_end("seek_from_end = \"0\"")]
    #[case::seek_from_start("seek_from_start = \"0\"")]
    #[case::magic("magic = b\"\\x01\"")]
    fn a_disqualifying_attribute_keeps_a_field_out_of_a_run(#[case] attr: &str) {
        let src =
            format!(r#"#[deku(endian = "big")] struct Test {{ #[deku({attr})] a: u8, b: u8 }}"#);
        assert_eq!(
            plan(&src),
            vec![],
            "`{attr}` must keep the field out of a run"
        );
    }

    #[test]
    fn the_id_storage_field_is_never_part_of_a_run() {
        // With `use_id`, the first field holds the enum id that has already been
        // read, so it is not a read at all and cannot join the run behind it.
        let src = &be_struct(&[2, 3, 3]);
        assert_eq!(plan_with_id(src, false), vec![(0, vec![2, 3, 3])]);
        assert_eq!(plan_with_id(src, true), vec![(1, vec![3, 3])]);

        // And with only one field left behind the id, there is no run at all.
        let src = &be_struct(&[2, 6]);
        assert_eq!(plan_with_id(src, true), vec![]);
    }

    #[test]
    fn a_run_forms_inside_an_enum_variant() {
        let src = r#"
        #[deku(id_type = "u8", endian = "big")]
        enum Test {
            #[deku(id = 1)]
            Named {
                #[deku(bits = 2)] a: u8,
                #[deku(bits = 6)] b: u8,
            },
            #[deku(id = 2)]
            Unnamed(#[deku(bits = 4)] u8, #[deku(bits = 4)] u8),
        }"#;
        assert_eq!(plan_variant(src, 0, false), vec![(0, vec![2, 6])]);
        // Unnamed fields take a different ident path but plan the same.
        assert_eq!(plan_variant(src, 1, false), vec![(0, vec![4, 4])]);
    }

    #[test]
    fn a_run_forms_in_a_tuple_struct() {
        let src = r#"#[deku(endian = "big")] struct Test(
            #[deku(bits = 3)] u8,
            #[deku(bits = 5)] u8,
        );"#;
        assert_eq!(plan(src), vec![(0, vec![3, 5])]);
    }

    #[test]
    fn update_does_not_keep_a_field_out_of_a_run() {
        // `update` is consumed only by the `DekuUpdate` impl, so it cannot change
        // how the field is read or written and must not split a run.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4, update = "0")] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);
    }

    #[test]
    fn the_emitted_read_makes_one_call_for_the_whole_run() {
        // The assertion the round-trip tests cannot make: three fields, one call.
        let data = DekuData::from_input(be_struct(&[2, 3, 3]).parse().unwrap()).unwrap();
        let emitted = emit_deku_read(&data).unwrap().to_string();
        assert_eq!(emitted.matches("read_bits_uint_msb0").count(), 1);

        // And without a run, one call per field.
        let src = r#"#[deku(endian = "little")] struct Test {
            #[deku(bits = 2)] a: u8,
            #[deku(bits = 3)] b: u8,
            #[deku(bits = 3)] c: u8,
        }"#;
        let data = DekuData::from_input(src.parse().unwrap()).unwrap();
        let emitted = emit_deku_read(&data).unwrap().to_string();
        assert_eq!(emitted.matches("read_bits_uint_msb0").count(), 0);
    }
}
