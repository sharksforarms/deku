use std::convert::TryFrom;

use darling::ast::Data;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

use crate::{DekuData, DekuDataEnum, DekuDataStruct, FieldData};

pub(crate) fn emit_deku_size(input: &DekuData) -> Result<TokenStream, syn::Error> {
    // NOTE: This is overkill, but keeps the same validating requirements of DekuRead. This ensures
    // the required values are set so we calculate the correct size.
    // TODO: We only care about validating and not the codegen. Split the two.
    let _ = super::deku_read::emit_deku_read(input)?;

    match &input.data {
        Data::Enum(_) => emit_enum(input),
        Data::Struct(_) => emit_struct(input),
    }
}

/// Calculate the size of a collection of fields
fn calculate_fields_size<'a>(
    fields: impl IntoIterator<Item = &'a FieldData>,
    crate_: &syn::Ident,
) -> TokenStream {
    let field_sizes = fields.into_iter().filter_map(|f| {
        if !f.temp {
            let field_type = &f.ty;

            #[cfg(feature = "bits")]
            if let Some(bits) = &f.bits {
                return Some(quote! { (#bits) });
            }

            if let Some(bytes) = &f.bytes {
                return Some(quote! { (#bytes) * 8 });
            }

            Some(quote! { <#field_type as ::#crate_::DekuSize>::SIZE_BITS })
        } else {
            None
        }
    });

    quote! { 0 #(+ #field_sizes)* }
}

/// Check if struct/enum has seek attributes
fn has_seek_attributes(input: &DekuData) -> bool {
    input.seek_rewind
        || input.seek_from_current.is_some()
        || input.seek_from_end.is_some()
        || input.seek_from_start.is_some()
}

/// Check if field has seek attributes
fn field_has_seek_attributes(field: &FieldData) -> bool {
    field.seek_rewind
        || field.seek_from_current.is_some()
        || field.seek_from_end.is_some()
        || field.seek_from_start.is_some()
}

/// Add DekuSize trait bounds to where clause for fields that need them
fn add_field_bounds<'a>(
    where_clause: &mut Option<syn::WhereClause>,
    fields: impl IntoIterator<Item = &'a FieldData>,
    crate_: &syn::Ident,
) {
    for field in fields {
        if !field.temp {
            let field_type = &field.ty;
            #[cfg(feature = "bits")]
            let needs_bound = field.bits.is_none() && field.bytes.is_none();
            #[cfg(not(feature = "bits"))]
            let needs_bound = field.bytes.is_none();

            if needs_bound {
                let where_clause = where_clause.get_or_insert_with(|| syn::parse_quote! { where });
                where_clause.predicates.push(syn::parse_quote! {
                    #field_type: ::#crate_::DekuSize
                });
            }
        }
    }
}

/// Calculate the discriminant size for an enum
fn calculate_discriminant_size(
    input: &DekuData,
    id: Option<&crate::Id>,
    id_type: Option<&TokenStream>,
    crate_: &syn::Ident,
) -> TokenStream {
    #[cfg(feature = "bits")]
    if let Some(bits) = &input.bits {
        return quote! { (#bits) };
    }

    if let Some(bytes) = &input.bytes {
        return quote! { (#bytes) * 8 };
    }

    if id.is_some() {
        return quote! { 0 };
    }

    if let Some(id_type) = id_type {
        return quote! { <#id_type as ::#crate_::DekuSize>::SIZE_BITS };
    }

    // This unwrap is ensured by us calling DekuRead validation
    let repr = &input.repr.unwrap();
    let repr_type = TokenStream::from(*repr);
    quote! { <#repr_type as ::#crate_::DekuSize>::SIZE_BITS }
}

fn emit_struct(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();

    if has_seek_attributes(input) {
        return Err(syn::Error::new(
            input.ident.span(),
            "DekuSize cannot be derived for types with seek attributes (seek_rewind, seek_from_current, seek_from_end, seek_from_start). Seek operations make size unpredictable.",
        ));
    }

    let DekuDataStruct {
        imp: _,
        wher: _,
        ident: _,
        fields,
    } = DekuDataStruct::try_from(input)?;

    let size_calculation = calculate_fields_size(fields.iter().copied(), &crate_);

    let (imp_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut where_clause = where_clause.cloned();
    add_field_bounds(&mut where_clause, fields.iter().copied(), &crate_);

    let ident = &input.ident;

    let tokens = quote! {
        impl #imp_generics ::#crate_::DekuSize for #ident #ty_generics #where_clause {
            const SIZE_BITS: usize = #size_calculation;
        }
    };

    Ok(tokens)
}

fn emit_enum(input: &DekuData) -> Result<TokenStream, syn::Error> {
    let crate_ = super::get_crate_name();

    if has_seek_attributes(input) {
        return Err(syn::Error::new(
            input.ident.span(),
            "DekuSize cannot be derived for types with seek attributes (seek_rewind, seek_from_current, seek_from_end, seek_from_start). Seek operations make size unpredictable.",
        ));
    }

    let DekuDataEnum {
        imp: _,
        wher: _,
        variants,
        ident: _,
        id,
        id_type,
        id_args: _,
    } = DekuDataEnum::try_from(input)?;

    let discriminant_size = calculate_discriminant_size(input, id, id_type, &crate_);

    let variant_sizes = variants
        .iter()
        .map(|variant| calculate_fields_size(variant.fields.iter(), &crate_));

    let max_variant_size = quote! {
        {
            const fn const_max(a: usize, b: usize) -> usize {
                if a > b { a } else { b }
            }

            let mut max = 0;
            #(
                max = const_max(max, #variant_sizes);
            )*
            max
        }
    };

    let (imp_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut where_clause = where_clause.cloned();
    for variant in variants.iter() {
        add_field_bounds(&mut where_clause, variant.fields.iter(), &crate_);
    }

    let ident = &input.ident;

    let tokens = quote! {
        impl #imp_generics ::#crate_::DekuSize for #ident #ty_generics #where_clause {
            const SIZE_BITS: usize = #discriminant_size + #max_variant_size;
        }
    };

    Ok(tokens)
}
