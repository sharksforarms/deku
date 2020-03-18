use crate::helpers::{extract_meta, MetaIteratorHelpers};
use proc_macro2::TokenStream;
use std::str::FromStr;
use std::string::ParseError;
use syn;
use syn::{Data, Fields, Type};

impl FromStr for EndianNess {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if ["big", "be", "network", "be"].contains(&s.to_lowercase().as_str()) {
            Ok(EndianNess::Big)
        } else if ["little", "le"].contains(&s.to_lowercase().as_str()) {
            Ok(EndianNess::Little)
        } else {
            panic!(format!("Cannot parse endian: {}", s));
        }
    }
}

#[derive(Clone, PartialEq)]
enum EndianNess {
    Big,
    Little,
}

pub fn impl_deku_read(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => panic!("deku only implemented for named fields"),
        },
        _ => panic!("deku only implemented for structs"),
    };

    let mut field_reads = Vec::new();
    let mut field_writes = Vec::new();
    for f in fields.named.iter() {
        let meta = extract_meta(&f.attrs);

        #[cfg(target_endian = "little")]
        let native_endian: EndianNess = EndianNess::from_str("little").unwrap();

        #[cfg(target_endian = "big")]
        let native_endian: EndianNess = EndianNess::from_str("big").unwrap();

        // get list of attributes
        let prop_bytes = meta
            .find_unique_property("deku", "bytes")
            .map(|v| v.parse::<usize>().unwrap());
        let prop_bits = meta
            .find_unique_property("deku", "bits")
            .map(|v| v.parse::<usize>().unwrap());
        let prop_endian = meta
            .find_unique_property("deku", "endian")
            .map(|v| EndianNess::from_str(v.as_str()).unwrap())
            .unwrap_or(native_endian.clone());

        let ident = f.ident.as_ref().unwrap();
        let type_ident = match &f.ty {
            Type::Path(ty) => ty.path.get_ident(),
            _ => panic!("err"),
        };

        if prop_bytes.is_some() && prop_bits.is_some() {
            panic!("err");
        }

        let prop_bits = prop_bytes.map(|v| v * 8).or(prop_bits);

        let bit_num = if prop_bits.is_some() {
            quote! { #prop_bits }
        } else {
            quote! { #type_ident::bit_size() }
        };

        let endian_flip = native_endian != prop_endian;

        let field_read = quote! {
            #ident: {
                let (ret_idx, res) = #type_ident::read(idx, #bit_num);
                let res = if (#endian_flip) {
                    res.swap_bytes()
                } else {
                    res
                };
                idx = ret_idx;
                res
            }
        };

        field_reads.push(field_read);

        let field_write = quote! {
            // Reverse to write from MSB -> LSB
            for i in (0..#bit_num).rev() {
                let field_val = if (#endian_flip) {
                    input.#ident.swap_bytes()
                } else {
                    input.#ident
                };

                let bit = (field_val & 1 << i) != 0;
                acc.push(bit)
            }
        };

        field_writes.push(field_write);
    }

    let ret = quote! {
        impl From<&[u8]> for #name {
            fn from(input: &[u8]) -> Self {
                let mut idx = (input, 0usize);
                Self {
                    #(#field_reads),*
                }
            }
        }

        impl From<#name> for Vec<u8> {
            fn from(input: #name) -> Self {
                let mut acc: BitVec<Msb0, u8> = BitVec::new();
                #(#field_writes)*

                acc.into_vec()
            }
        }
    };

    println!("{}", ret.to_string());

    ret
}
