use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;

pub(crate) mod deku_read;
pub(crate) mod deku_write;

/// Attempts to extract the concrete item type from a Vec (e.g Vec<u8> -> u8)
fn extract_vec_item_type(field_type: &syn::Type) -> Option<&syn::Type> {
    let path = match field_type {
        syn::Type::Path(syn::TypePath { path, .. }) => path,
        _ => return None,
    };
    if path.segments.len() != 1 {
        return None;
    }
    let seg = &path.segments[0];

    // Make sure its a vec
    if seg.ident.to_string() != "Vec" {
        return None;
    }

    // Extract the generic item
    let arg = match seg.arguments {
        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            ref args,
            ..
        }) => {
            if args.len() != 1 {
                return None;
            }
            &args[0]
        }
        _ => return None,
    };

    // It was a type, not lifetime, etc...
    if let syn::GenericArgument::Type(t) = arg {
        return Some(t);
    }

    None
}

/// Validates the use of deku(vec_len) and returns the vec_ident on success.
/// This will update existing_fields on success and fixup field_type/field_ident to
/// point to the Vec's item type
fn validate_vec_len(
    field_ident: &mut TokenStream,
    field_type: &mut &syn::Type,
    len_field_name: &str,
    existing_fields: &mut HashSet<String>,
) -> Result<TokenStream, darling::Error> {
    let vec_ident;

    // The field containing the length must have been parsed already
    if !existing_fields.contains(len_field_name) {
        // TODO : Create real error for this
        return Err(darling::Error::duplicate_field(
            "deku(vec_len) references an invalid field",
        ));
    }

    // field_type now points to the type of item the Vec is holding
    *field_type = match extract_vec_item_type(field_type) {
        Some(t) => t,
        None => {
            return Err(darling::Error::duplicate_field(
                "Unable to extract vector type for deku(vec_len) field",
            ))
        }
    };

    // Rename field_ident to [vec_field]_tmp to use inside the loop
    vec_ident = field_ident.clone();
    let tmp_field_ident = syn::Ident::new(
        &format!("{}_tmp", quote! { #field_ident }),
        syn::export::Span::call_site(),
    );
    existing_fields.insert(field_ident.to_string());

    *field_ident = quote! {
        #tmp_field_ident
    };

    // Return the
    Ok(vec_ident)
}
