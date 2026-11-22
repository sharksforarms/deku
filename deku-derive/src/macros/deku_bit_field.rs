use darling::ast::{Data, Fields};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;

use super::{
    BitRunField, extract_run_field, gen_enum_init, gen_internal_field_ident,
    gen_internal_field_idents, gen_struct_init, low_mask, no_variant_error, run_field, shift_of,
    sum_bits, unsigned_width,
};
use crate::{DekuData, FieldData, Id, Num, VariantData};

/// `DekuBitField` for a type whose wire form is one run of bits: a struct of
/// run fields, or an enum read by its id and then the run fields of its
/// variant.
///
/// A field takes the same rules as a field of a struct's run: a literal
/// `bits` on an unsigned primitive or `bool`, or a `bit_field`, with literal
/// padding and nothing else. Anything that needs the reader itself is a
/// compile error rather than a missing impl, so the type's author learns
/// here, not at a struct using it.
pub(crate) fn emit_deku_bit_field(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let ident = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(error(
            input.generics.span(),
            "generic types are not supported",
        ));
    }
    if input.any_container_set_incompatible_with_bit_field() {
        return Err(error(
            ident.span(),
            "`magic`, `bytes` and `seek_*` need the reader, not just the bits",
        ));
    }
    // `Msb0` is the default, so absent is fine; "lsb" or a ctx order is not.
    if let Some(order) = &input.bit_order
        && order.value() != "msb"
    {
        return Err(error(order.span(), "only `Msb0` lines up with a bit run"));
    }

    match &input.data {
        Data::Struct(fields) => emit_struct(input, fields),
        Data::Enum(variants) => emit_enum(input, variants),
    }
}

/// The bits of a struct are its fields, first field highest.
fn emit_struct(input: &DekuData, fields: &Fields<FieldData>) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();
    let ident = &input.ident;
    if input.id.is_some() || input.id_type.is_some() {
        return Err(error(ident.span(), "`id` and `id_type` are for enums"));
    }

    let fields = fields.as_ref();
    let run = plan(input, &fields)?;
    let width = width_of(&run, &crate_);
    let from = emit_from(&fields, &run, &crate_);
    let init = gen_struct_init(fields.style.is_struct(), init_idents(&fields));
    let destruct = pattern(quote! { Self }, &fields);
    let to = emit_to(&fields, &run, &crate_);

    Ok(quote! {
        #[automatically_derived]
        impl ::#crate_::DekuBitField for #ident {
            const BITS: usize = {
                let __deku_bits = #width;
                assert!(
                    __deku_bits >= 1 && __deku_bits <= 64,
                    "DekuBitField: a struct must have from 1 to 64 bits",
                );
                __deku_bits
            };

            #[inline]
            fn from_bit_run(__deku_raw: u64) -> core::result::Result<Self, ::#crate_::DekuError> {
                #from
                Ok(#init)
            }

            #[inline]
            fn to_bit_run(&self) -> core::result::Result<u64, ::#crate_::DekuError> {
                let #destruct = self;
                #to
            }
        }
    })
}

/// The bits of an enum are its id, then the fields of the variant below it.
/// Each variant takes the same number of bits, so that a run holding the
/// enum is one width whatever the variant.
fn emit_enum(input: &DekuData, variants: &[VariantData]) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();
    let ident = &input.ident;

    // With `id` the discriminant comes from context, not off the wire.
    if let Some(id) = &input.id {
        return Err(error(
            id.to_token_stream().span(),
            "`id` takes the discriminant from context, so there are no bits to convert",
        ));
    }
    let Some(id_type) = &input.id_type else {
        return Err(error(ident.span(), "`id_type` is required"));
    };
    let Some(type_width) = unsigned_width(&id_type.to_string()) else {
        return Err(error(
            id_type.span(),
            "`id_type` must be `u8`, `u16`, `u32` or `u64`",
        ));
    };
    let id_bits = match &input.bits {
        Some(Num::LitInt(lit)) => {
            let bits = lit.base10_parse::<usize>()?;
            if bits == 0 || bits > type_width {
                return Err(error(
                    lit.span(),
                    "`bits` must be between 1 and the width of `id_type`",
                ));
            }
            bits
        }
        Some(Num::TokenStream(ts)) => {
            return Err(error(ts.span(), "`bits` must be an integer literal"));
        }
        None => type_width,
    };
    // Past a byte, only big-endian lines up with a run. An endian from context
    // is fine, since runs only form in big-endian structs.
    if id_bits > 8 {
        let endian = input.id_endian.as_ref().or(input.endian.as_ref());
        if endian.is_none_or(|e| e.value() == "little") {
            return Err(error(
                ident.span(),
                "an id wider than 8 bits must be big-endian, or take its endian from context",
            ));
        }
    }

    // The width of the first variant, which every other must equal.
    let mut first_width: Option<(TokenStream, Option<usize>)> = None;
    let mut width_checks = Vec::new();
    let mut from_arms = Vec::with_capacity(variants.len());
    let mut to_arms = Vec::with_capacity(variants.len());
    for variant in variants {
        if variant.any_variant_set_incompatible_with_bit_field() {
            return Err(error(
                variant.ident.span(),
                "every variant needs a literal `id`, without `id_pat`, `reader`, `writer` or `default`",
            ));
        }
        // `id = 1` and `id = "1"` are the same literal, written two ways.
        let id = match &variant.id {
            Some(Id::Int(id)) => id.clone(),
            Some(Id::TokenStream(ts)) => match syn::parse2::<syn::LitInt>(ts.clone()) {
                Ok(id) => id,
                Err(_) => return Err(error(ts.span(), ID_MUST_BE_AN_INTEGER)),
            },
            _ => return Err(error(variant.ident.span(), ID_MUST_BE_AN_INTEGER)),
        };
        // The individual write would reject it at run time and no read could
        // ever produce it, so it can only be a mistake.
        let value = id.base10_parse::<u64>()?;
        if id_bits < 64 && value >> id_bits != 0 {
            return Err(error(
                id.span(),
                &format!("`id` {id} does not fit in {id_bits} bits"),
            ));
        }

        let fields = variant.fields.as_ref();
        let run = plan(input, &fields)?;
        let width = width_of(&run, &crate_);
        let lit_width: Option<usize> = run.iter().map(BitRunField::lit_span).sum();
        match &first_width {
            None => first_width = Some((width, lit_width)),
            Some((_, Some(first))) if lit_width.is_some_and(|lit| lit != *first) => {
                return Err(error(
                    variant.ident.span(),
                    &format!(
                        "every variant must have the same bits, but this one has {} and the first has {first}",
                        lit_width.unwrap_or_default(),
                    ),
                ));
            }
            Some((_, Some(_))) if lit_width.is_some() => {}
            // A `bit_field` makes the width a constant of a trait, which only
            // the compiler can compare.
            Some((first, _)) => width_checks.push(quote! {
                assert!(
                    #first == #width,
                    "DekuBitField: every variant must have the same bits",
                );
            }),
        }

        let variant_ident = &variant.ident;
        let named = fields.style.is_struct();
        let from = emit_from(&fields, &run, &crate_);
        let init = gen_enum_init(named, quote! { Self::#variant_ident }, init_idents(&fields));
        from_arms.push(quote! {
            #id => {
                #from
                #init
            }
        });

        let destruct = pattern(quote! { Self::#variant_ident }, &fields);
        let to = emit_to(&fields, &run, &crate_);
        to_arms.push(quote! {
            #destruct => {
                let __deku_payload: u64 = #to?;
                Ok(((#id as u64) << __DEKU_PAYLOAD_BITS) | __deku_payload)
            }
        });
    }

    // An enum with no variants has no value to hold: its own reader can only
    // fail, so it has no width to give a run.
    let Some((payload, _)) = first_width else {
        return Err(error(ident.span(), "an enum needs at least one variant"));
    };
    let id_mask = low_mask(id_bits);
    let no_variant = no_variant_error(&crate_, &ident.to_string());

    Ok(quote! {
        #[automatically_derived]
        impl ::#crate_::DekuBitField for #ident {
            const BITS: usize = {
                #(#width_checks)*
                let __deku_bits = #id_bits + #payload;
                assert!(
                    __deku_bits <= 64,
                    "DekuBitField: an enum must have at most 64 bits",
                );
                __deku_bits
            };

            #[inline]
            fn from_bit_run(__deku_raw: u64) -> core::result::Result<Self, ::#crate_::DekuError> {
                const __DEKU_PAYLOAD_BITS: usize = #payload;
                // `checked_shr` keeps an id of 64 bits from a shift by 64.
                let __deku_variant_id = (__deku_raw
                    .checked_shr(__DEKU_PAYLOAD_BITS as u32)
                    .unwrap_or(0)
                    & #id_mask) as #id_type;
                Ok(match __deku_variant_id {
                    #(#from_arms,)*
                    _ => return Err(#no_variant),
                })
            }

            #[inline]
            fn to_bit_run(&self) -> core::result::Result<u64, ::#crate_::DekuError> {
                const __DEKU_PAYLOAD_BITS: usize = #payload;
                match self {
                    #(#to_arms,)*
                }
            }
        }
    })
}

/// The run of `fields`, first field highest, or the reason one of them
/// cannot join it.
fn plan(input: &DekuData, fields: &Fields<&FieldData>) -> Result<Vec<BitRunField>, syn::Error> {
    fields
        .fields
        .iter()
        .map(|f| {
            if f.temp || f.skip.is_some() {
                return Err(error(
                    f.ty.span(),
                    "a `temp` or `skip` field holds no bits of its own",
                ));
            }
            run_field(input, f).ok_or_else(|| error(f.ty.span(), FIELD_MUST_BE_A_RUN_FIELD))
        })
        .collect()
}

/// The bits of `run`, padding included, as a constant expression.
fn width_of(run: &[BitRunField], crate_: &syn::Ident) -> TokenStream {
    if run.is_empty() {
        return quote! { 0usize };
    }
    let refs: Vec<&BitRunField> = run.iter().collect();
    sum_bits(&refs, crate_)
}

/// Binds each field of `run` to its internal ident, out of `__deku_raw`.
fn emit_from(fields: &Fields<&FieldData>, run: &[BitRunField], crate_: &syn::Ident) -> TokenStream {
    let refs: Vec<&BitRunField> = run.iter().collect();
    let raw = quote! { __deku_raw };
    let mut binds = TokenStream::new();
    for (offset, field) in run.iter().enumerate() {
        let internal = gen_internal_field_ident(&fields.fields[offset].get_ident(offset, true));
        let shift = shift_of(&refs, offset, crate_);
        let extract = extract_run_field(field, &raw, &shift, crate_);
        binds.extend(quote! { let #internal = #extract; });
    }
    binds
}

/// The bits of each field of `run`, with the check its own write does,
/// combined into one `Result<u64, DekuError>`. The fields are bound by
/// reference, as `pattern` binds them.
fn emit_to(fields: &Fields<&FieldData>, run: &[BitRunField], crate_: &syn::Ident) -> TokenStream {
    let refs: Vec<&BitRunField> = run.iter().collect();
    let mut checks = TokenStream::new();
    let mut terms = Vec::with_capacity(run.len());
    for (offset, field) in run.iter().enumerate() {
        let field_ident = fields.fields[offset].get_ident(offset, true);
        let shift = shift_of(&refs, offset, crate_);
        let mask = field.mask(crate_);
        let value = match field {
            BitRunField::Primitive {
                bits,
                ordered,
                can_overflow,
                ..
            } => {
                let value = quote! { (*#field_ident as u64) };
                if *can_overflow {
                    checks.extend(quote! {
                        ::#crate_::writer::check_bit_size::<#ordered>(#value, #bits)?;
                    });
                }
                value
            }
            BitRunField::BitField { ty, .. } => {
                let value = quote::format_ident!("__deku_bit_field_{}", offset);
                checks.extend(quote! {
                    let #value: u64 = <#ty as ::#crate_::DekuBitField>::to_bit_run(#field_ident)?;
                });
                quote! { #value }
            }
        };
        terms.push(quote! { ((#value & #mask) << #shift) });
    }
    quote! {
        (|| -> core::result::Result<u64, ::#crate_::DekuError> {
            #checks
            Ok(0u64 #(| #terms)*)
        })()
    }
}

/// A pattern of `path` that binds each field, by reference where it matches
/// a reference, to the ident `field_idents` gives it.
fn pattern(path: TokenStream, fields: &Fields<&FieldData>) -> TokenStream {
    let idents = field_idents(fields);
    if fields.style.is_unit() {
        path
    } else if fields.style.is_struct() {
        quote! { #path { #(#idents),* } }
    } else {
        quote! { #path ( #(#idents),* ) }
    }
}

/// The idents `pattern` binds, in field order.
fn field_idents(fields: &Fields<&FieldData>) -> Vec<TokenStream> {
    fields
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| f.get_ident(i, true))
        .collect()
}

/// The fields of a struct or variant initializer, from their internal idents.
fn init_idents(fields: &Fields<&FieldData>) -> std::vec::IntoIter<TokenStream> {
    let idents = field_idents(fields);
    gen_internal_field_idents(fields.style.is_struct(), idents.iter())
        .collect::<Vec<_>>()
        .into_iter()
}

const ID_MUST_BE_AN_INTEGER: &str =
    "every variant needs an integer `id`, such as `id = 1` or `id = \"0x01\"`";

const FIELD_MUST_BE_A_RUN_FIELD: &str = "every field needs a literal `bits` on an unsigned \
     integer or `bool`, or `bit_field`, in big-endian `Msb0`, with nothing else but literal \
     padding";

fn error(span: Span, msg: &str) -> syn::Error {
    syn::Error::new(span, format!("DekuBitField: {msg}"))
}
