pub(crate) mod deku_read;
pub(crate) mod deku_write;


fn extract_vec_generic(field_type: &syn::Type) -> Option<&syn::Type> {
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

    return None;
}