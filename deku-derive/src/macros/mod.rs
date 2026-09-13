use darling::ast::Fields;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::Lifetime;
#[cfg(feature = "bits")]
use syn::LitStr;

use crate::{DekuData, FieldData, Num};

pub(crate) mod deku_read;
pub(crate) mod deku_size;
pub(crate) mod deku_write;

#[cfg(feature = "proc-macro-crate")]
fn get_crate_name() -> Ident {
    let found_crate =
        proc_macro_crate::crate_name("deku").unwrap_or(proc_macro_crate::FoundCrate::Itself);

    let crate_name = match found_crate {
        proc_macro_crate::FoundCrate::Itself => "deku".to_string(),
        proc_macro_crate::FoundCrate::Name(name) => name,
    };

    Ident::new(&crate_name, Span::call_site())
}

// proc-macro-crate depends on std, for no_std, use default name. Sorry.
#[cfg(not(feature = "proc-macro-crate"))]
fn get_crate_name() -> Ident {
    Ident::new("deku", Span::call_site())
}

/// Generate enum initialization TokenStream
/// Cases:
/// - No fields: `MyEnum`
/// - Named: `MyEnum { field_idents }`
/// - Unnamed:  `MyEnum ( field_idents )`
fn gen_enum_init<V: ToTokens, I: ToTokens>(
    is_named: bool,
    enum_variant: V,
    field_idents: impl Iterator<Item = I>,
) -> TokenStream {
    let mut field_idents = field_idents.peekable();
    if field_idents.peek().is_none() {
        return quote! { #enum_variant };
    }

    if is_named {
        quote! {
            #enum_variant {
                #(#field_idents),*
            }
        }
    } else {
        quote! {
            #enum_variant (
                #(#field_idents),*
            )
        }
    }
}

/// Generate struct initialization TokenStream
/// Cases:
/// - No fields: `Self {}`
/// - Named: `Self { field_idents }`
/// - Unnamed:  `Self ( field_idents )`
fn gen_struct_init<I: ToTokens>(
    is_named: bool,
    field_idents: impl Iterator<Item = I>,
) -> TokenStream {
    let mut field_idents = field_idents.peekable();
    if field_idents.peek().is_none() {
        return quote! { Self {} };
    }

    if is_named {
        quote! {
            Self {
                #(#field_idents),*
            }
        }
    } else {
        quote! {
            Self (
                #(#field_idents),*
            )
        }
    }
}

/// Generate struct destruction
///
/// - Named: `#ident { ref fields }`
/// - Unnamed: `#ident ( ref fields )`
fn gen_struct_destruction<I: ToTokens, F: ToTokens>(
    named: bool,
    unit: bool,
    ident: I,
    field_idents: impl Iterator<Item = F>,
) -> TokenStream {
    if unit {
        quote! {
            #ident
        }
    } else if named {
        quote! {
            #ident {
                #(ref #field_idents),*
            }
        }
    } else {
        quote! {
            #ident (
                #(ref #field_idents),*
            )
        }
    }
}

/// Convert a field ident to internal ident:
/// `a` -> `__deku_a`
fn gen_internal_field_ident(ident: &TokenStream) -> TokenStream {
    // Concat token: https://github.com/rust-lang/rust/issues/29599
    let span = ident.span();
    let s = ident.to_string();
    let mut name = "__deku___".to_owned();
    // If its a raw identifier, we must remove 'r#'
    name.push_str(s.strip_prefix("r#").unwrap_or(&s));

    syn::Ident::new(&name, span).to_token_stream()
}

/// Map all field indents to internal idents
///
/// - Named: `{ a: __deku_a }`
/// - Unnamed: `( __deku_a )`
fn gen_internal_field_idents<'a>(
    named: bool,
    idents: impl Iterator<Item = &'a TokenStream> + 'a,
) -> impl Iterator<Item = TokenStream> + 'a {
    idents.map(move |i| {
        if named {
            let h = gen_internal_field_ident(i);
            quote! {#i: #h}
        } else {
            gen_internal_field_ident(i)
        }
    })
}

fn split_ctx_to_pats_and_types(
    ctx: &Punctuated<syn::FnArg, syn::token::Comma>,
) -> syn::Result<Vec<(&syn::Pat, &syn::Type)>> {
    // `()` or `(u8, u32)`
    ctx.iter()
        .map(|arg| {
            match arg {
                syn::FnArg::Typed(pat_type) => Ok((pat_type.pat.as_ref(), pat_type.ty.as_ref())),
                // a self is unacceptable
                syn::FnArg::Receiver(r) => Err(syn::Error::new(r.span(), "Unacceptable context")),
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

/// Generate ctx types and argument
///
/// - Empty: arg: `(): ()`, type: `()`
/// - One: arg: `a: usize`, type: `usize`
/// - Other: arg: `(a, b, ...): (u8, u8, ...)`, type: `(u8, u8, ...)`
fn gen_ctx_types_and_arg(
    ctx: Option<&Punctuated<syn::FnArg, syn::token::Comma>>,
) -> syn::Result<(TokenStream, TokenStream)> {
    if let Some(ctx) = ctx {
        let pats_types = split_ctx_to_pats_and_types(ctx)?;

        if pats_types.len() == 1 {
            // remove parens for single item
            let (pat, ty) = pats_types[0];
            Ok((quote! {#ty}, quote! {#pat:#ty}))
        } else {
            let pats = pats_types.iter().map(|(pat, _)| pat);
            let types = pats_types.iter().map(|(_, ty)| ty);

            // "a: u8, b: usize" -> (u8, usize)
            let types_cpy = types.clone();
            let ctx_types = quote! {(#(#types_cpy),*)};
            // "a: u8, b: usize" -> (a, b): (u8, usize)
            let ctx_arg = quote! {(#(#pats),*): (#(#types),*)};

            Ok((ctx_types, ctx_arg))
        }
    } else {
        Ok((quote! {()}, quote! {_: ()}))
    }
}

/// Generate type from matching ident from `id` in `ctx`
///
/// - #[deku(ctx = "test: u16, my_id: u8", id = "my_id")], will return `u8`
/// - #[deku(ctx = "test: u16, my_id: u8", id = "my_id, test")], will return `u8, u16`
fn gen_type_from_ctx_id(
    ctx: &Punctuated<syn::FnArg, syn::token::Comma>,
    id: &crate::Id,
) -> Option<TokenStream> {
    let parser = Punctuated::<Ident, Comma>::parse_terminated;
    let Ok(s) = parser.parse(id.to_token_stream().into()) else {
        return None;
    };
    let mut matching_types = quote! {};
    for s in s {
        let id = syn::Ident::new(&s.to_string(), id.span());

        let types = ctx.iter().find_map(|arg| {
            let mut t = None;
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    if id == ident.ident {
                        let mut pat_type = pat_type.clone();
                        if let syn::Type::Reference(r) = pat_type.ty.as_mut() {
                            r.lifetime = Some(Lifetime::new("'__deku", Span::call_site()));
                        }
                        let ty = &pat_type.ty;
                        t = Some(quote! {#ty});
                    }
                }
            }

            t
        });
        if matching_types.is_empty() {
            matching_types = quote! {#matching_types #types};
        } else {
            matching_types = quote! {#matching_types, #types};
        }
    }

    if matching_types.is_empty() {
        None
    } else {
        Some(matching_types)
    }
}

/// Generate argument for `id`:
/// `#deku(endian = "big", bits = 1)` -> `Endian::Big, BitSize(1)`
/// `#deku(endian = "big", bytes = 1)` -> `Endian::Big, ByteSize(1)`
pub(crate) fn gen_id_args(
    endian: Option<&syn::LitStr>,
    id_endian: Option<&syn::LitStr>,
    bits: Option<&Num>,
    bytes: Option<&Num>,
    bit_order: Option<&syn::LitStr>,
) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    let endian = id_endian
        .map(gen_endian_from_str)
        .or_else(|| endian.map(gen_endian_from_str))
        .transpose()?;
    let bits = bits.map(|n| quote! {::#crate_::ctx::BitSize(#n)});
    let bytes = bytes.map(|n| quote! {::#crate_::ctx::ByteSize(#n)});
    let bit_order = bit_order.map(gen_bit_order_from_str).transpose()?;

    // FIXME: Should be `into_iter` here, see https://github.com/rust-lang/rust/issues/66145.
    let id_args = [
        endian.as_ref(),
        bits.as_ref(),
        bytes.as_ref(),
        bit_order.as_ref(),
    ]
    .iter()
    .filter_map(|i| *i)
    .collect::<Vec<_>>();

    match &id_args[..] {
        [arg] => Ok(quote! {#arg}),
        args => Ok(quote! {#(#args),*}),
    }
}

/// Generate argument for fields:
///
/// `#deku(endian = "big", bits = 1, ctx = "a")` -> `Endian::Big, BitSize(1), a`
/// `#deku(endian = "big", bytes = 1, ctx = "a")` -> `Endian::Big, ByteSize(1), a`
fn gen_field_args(
    endian: Option<&syn::LitStr>,
    bits: Option<&Num>,
    bytes: Option<&Num>,
    ctx: Option<&Punctuated<syn::Expr, syn::token::Comma>>,
    bit_order: Option<&syn::LitStr>,
) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    let endian = endian.map(gen_endian_from_str).transpose()?;
    let bits = bits.map(|n| quote! {::#crate_::ctx::BitSize(#n)});
    let bytes = bytes.map(|n| quote! {::#crate_::ctx::ByteSize(#n)});
    let bit_order = bit_order.map(gen_bit_order_from_str).transpose()?;
    let ctx = ctx.map(|c| quote! {#c});

    // FIXME: Should be `into_iter` here, see https://github.com/rust-lang/rust/issues/66145.
    // TODO: the order here should be documented
    let field_args = [
        endian.as_ref(),
        bits.as_ref(),
        bytes.as_ref(),
        bit_order.as_ref(),
        ctx.as_ref(),
    ]
    .iter()
    .filter_map(|i| *i)
    .collect::<Vec<_>>();

    // Because `impl DekuRead<'_, (T1, T2)>` but `impl DekuRead<'_, T1>`(not tuple)
    match &field_args[..] {
        [arg] => Ok(quote! {#arg}),
        args => Ok(quote! {#(#args),*}),
    }
}

/// Generate bit_order tokens from string: `lsb` -> `Order::Lsb0`.
fn gen_bit_order_from_str(s: &syn::LitStr) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    match s.value().as_str() {
        "lsb" => Ok(quote! {::#crate_::ctx::Order::Lsb0}),
        "msb" => Ok(quote! {::#crate_::ctx::Order::Msb0}),
        _ => {
            // treat as variable, possibly from `ctx`
            let v: TokenStream = s.value().parse()?;
            Ok(quote! {#v})
        }
    }
}

/// Generate endian tokens from string: `big` -> `Endian::Big`.
fn gen_endian_from_str(s: &syn::LitStr) -> syn::Result<TokenStream> {
    let crate_ = get_crate_name();
    match s.value().as_str() {
        "little" => Ok(quote! {::#crate_::ctx::Endian::Little}),
        "big" => Ok(quote! {::#crate_::ctx::Endian::Big}),
        _ => {
            // treat as variable, possibly from `ctx`
            let v: TokenStream = s.value().parse()?;
            Ok(quote! {#v})
        }
    }
}

/// Wraps a TokenStream with a closure providing access to `ctx` variables when
/// `ctx_default` is provided
fn wrap_default_ctx(
    body: TokenStream,
    ctx: &Option<syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>>,
    ctx_default: &Option<Punctuated<syn::Expr, syn::token::Comma>>,
) -> TokenStream {
    if let (Some(ctx), Some(ctx_default)) = (ctx, ctx_default) {
        // wrap in a function to make `ctx` variables in scope
        quote! {
            |#ctx| -> ::core::result::Result<_, _> {
                #body
            }(#ctx_default)
        }
    } else {
        body
    }
}

/// Returns true if the literal substring `s` is in the token
fn token_contains_string(tok: &Option<TokenStream>, s: &str) -> bool {
    tok.as_ref()
        .map(|v| {
            let v = v.to_string();
            v.contains(s)
        })
        .unwrap_or(false)
}

#[cfg(feature = "bits")]
fn pad_bits(
    bits: Option<&TokenStream>,
    bytes: Option<&TokenStream>,
    bit_order: Option<&LitStr>,
    emit_padding: fn(&TokenStream, bit_order: Option<&LitStr>) -> TokenStream,
) -> TokenStream {
    match (bits, bytes) {
        (Some(pad_bits), Some(pad_bytes)) => {
            emit_padding(&quote! { (#pad_bits) + ((#pad_bytes) * 8) }, bit_order)
        }
        (Some(pad_bits), None) => emit_padding(pad_bits, bit_order),
        (None, Some(pad_bytes)) => emit_padding(&quote! {((#pad_bytes) * 8)}, bit_order),
        (None, None) => quote!(),
    }
}

#[cfg(not(feature = "bits"))]
fn pad_bytes(
    bytes: Option<&TokenStream>,
    emit_padding: fn(&TokenStream) -> TokenStream,
) -> TokenStream {
    match bytes {
        Some(pad_bytes) => emit_padding(&quote! {((#pad_bytes))}),
        None => quote!(),
    }
}

/// assertion is false, raise error
fn assertion_failed(
    v: &TokenStream,
    ident: &str,
    field_ident_str: &str,
    field_ident: Option<&TokenStream>,
) -> TokenStream {
    let crate_ = get_crate_name();
    let stringify = if let Some(field_ident) = field_ident {
        quote! { stringify!(#field_ident == #v) }
    } else {
        quote! { stringify!(#v) }
    };
    {
        quote! {
            return Err(::#crate_::deku_error!(::#crate_::DekuError::Assertion, "Field failed assertion", "{}.{}: {}", #ident, #field_ident_str, #stringify));
        }
    }
}

/// One field of a contiguous big-endian `Msb0` bit-field run.
#[cfg(feature = "bits")]
pub(crate) struct BitRunField {
    pub(crate) bits: usize,
    pub(crate) ty: syn::Type,
    /// Field takes the `Order`-carrying write impl, which words overflow
    /// differently.
    pub(crate) ordered: bool,
    /// Whether a value can exceed `bits` at all. If not, no check is emitted.
    pub(crate) can_overflow: bool,
    /// Whether the field takes its type's whole width, so it is a plain byte
    /// field rather than a packed one. A run of only these is already served by
    /// the byte path.
    pub(crate) whole_width: bool,
}

/// Widths of a run of adjacent fields that one read can serve.
#[cfg(feature = "bits")]
pub(crate) type BitRun = Vec<BitRunField>;

/// A plain `bool`, which a run compares rather than casts.
#[cfg(feature = "bits")]
fn is_bool(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.qself.is_none() && p.path.is_ident("bool"))
}

/// A field a run can serve: a literal `bits` on an unsigned primitive or `bool`,
/// explicitly big-endian, `Msb0`, carrying nothing else. Anything else keeps its
/// own read.
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

    // Only `Msb0` batches: absent is the default and fine, "lsb" is not, and
    // anything else is a ctx parameter name forwarded as a runtime order, which
    // could be either at run time.
    let explicit_order = f.bit_order.as_ref().or(input.bit_order.as_ref());
    if let Some(order) = explicit_order {
        if order.value() != "msb" {
            return None;
        }
    }
    // Which overflow wording this field's own write would have used.
    let ordered = explicit_order.is_some();

    let width = match &f.ty {
        syn::Type::Path(p) if p.qself.is_none() => match p.path.get_ident()?.to_string().as_str() {
            "u8" => u8::BITS as usize,
            "u16" => u16::BITS as usize,
            "u32" => u32::BITS as usize,
            "u64" => u64::BITS as usize,
            // `impls::bool` delegates to `u8`, so a bool is a byte unless
            // `bits` narrows it. Flags in a packed header are usually `bits = 1`.
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

    // Neither a value filling its type nor a bool can exceed its width.
    let can_overflow = bits < width && !is_bool(&f.ty);

    Some(BitRunField {
        bits,
        ty: f.ty.clone(),
        ordered,
        can_overflow,
        whole_width: bits == width,
    })
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
        // A run of only whole-width fields is a stretch of plain bytes. The byte
        // path already reads those in one call, whereas a run sends them through
        // the bit reader, which costs a `u64` accumulator and a shift and a mask
        // per field for no gain. Batch only when the run holds a packed field.
        let packed = run.iter().any(|f| !f.whole_width);
        if run.len() >= 2 && packed {
            let len = run.len();
            runs.insert(i, run);
            i += len;
        } else {
            i += 1;
        }
    }
    runs
}

#[cfg(test)]
#[cfg(feature = "bits")]
mod tests {
    use rstest::rstest;

    use crate::emit_deku_read;

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

    /// As `plan`, with `use_id`: the first field is an enum's id storage, not a read.
    fn plan_with_id(src: &str, use_id: bool) -> Vec<(usize, Vec<usize>)> {
        let data = DekuData::from_input(src.parse().unwrap()).expect("input should parse");
        let fields = data
            .data
            .as_ref()
            .take_struct()
            .expect("test input should be a struct");

        sorted(plan_bit_runs(&data, &fields, use_id))
    }

    /// The `whole_width` flag of every field of every run, keyed by start index.
    ///
    /// `plan` keeps only `bits`, which cannot show what a batched run carries into
    /// the emitters.
    fn plan_whole_width(src: &str) -> Vec<(usize, Vec<bool>)> {
        let data = DekuData::from_input(src.parse().unwrap()).expect("input should parse");
        let fields = data
            .data
            .as_ref()
            .take_struct()
            .expect("test input should be a struct");

        let mut runs: Vec<_> = plan_bit_runs(&data, &fields, false)
            .into_iter()
            .map(|(start, run)| {
                (
                    start,
                    run.iter().map(|f| f.whole_width).collect::<Vec<bool>>(),
                )
            })
            .collect();
        runs.sort_by_key(|(start, _)| *start);
        runs
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
        // One field costs the same read either way.
        assert_eq!(plan(&be_struct(&[5])), vec![]);
    }

    #[test]
    fn plain_fields_without_bits_are_their_full_width() {
        // A packed field opens the run, so the three plain ones join it at the
        // width of their type.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4)] p: u8,
            a: u8,
            b: u16,
            c: u32,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 8, 16, 32])]);
    }

    #[test]
    fn a_run_is_capped_at_64_bits_and_the_next_one_starts_there() {
        // 4 + 32 + 16 leaves 12 bits, which the fourth field cannot fill, so it
        // opens a second run with the packed field behind it.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4)] p: u8,
            a: u32,
            b: u16,
            c: u32,
            #[deku(bits = 4)] q: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 32, 16]), (3, vec![32, 4])]);

        // A field that does not fit closes the run rather than overflowing it.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4)] p: u8,
            a: u32,
            b: u32,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 32])]);
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
        // Absent means the target's endianness, little on x86.
        assert_eq!(
            plan(r#"struct Test { #[deku(bits = 4)] a: u8, #[deku(bits = 4)] b: u8 }"#),
            vec![]
        );
        assert_eq!(
            plan(
                r#"#[deku(endian = "little")] struct Test {
                #[deku(bits = 4)] a: u8,
                #[deku(bits = 4)] b: u8,
            }"#
            ),
            vec![]
        );
        // A field-level attribute qualifies a field inside a little-endian struct.
        let src = r#"#[deku(endian = "little")] struct Test {
            #[deku(endian = "big", bits = 4)] a: u8,
            #[deku(endian = "big", bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);
    }

    #[test]
    fn bit_order_must_be_msb() {
        // `Msb0` is the default, so absent qualifies, and so does spelling it out.
        assert_eq!(plan(&be_struct(&[4, 4])), vec![(0, vec![4, 4])]);

        let src = r#"#[deku(endian = "big", bit_order = "msb")] struct Test {
            #[deku(bits = 4)] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);

        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bit_order = "msb", bits = 4)] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);

        // "lsb" does not.
        let src = r#"#[deku(endian = "big", bit_order = "lsb")] struct Test {
            #[deku(bits = 4)] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![]);

        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bit_order = "lsb", bits = 4)] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![]);
    }

    #[test]
    fn a_runtime_bit_order_does_not_batch() {
        // An `Order`-typed `ctx` parameter is forwarded as `bit_order`, so the
        // order is only known at run time and could be `Lsb0`.
        let src = r#"#[deku(endian = "big", ctx = "order: deku::ctx::Order")] struct Test {
            #[deku(bits = 4)] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![]);

        // A wildcard binds nothing, so there is no runtime order to honour.
        let src = r#"#[deku(endian = "big", ctx = "_: deku::ctx::Order")] struct Test {
            #[deku(bits = 4)] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);
    }

    #[test]
    fn an_explicit_bit_order_selects_the_other_overflow_wording() {
        // Both batch, but reach different write impls, so each needs its own wording.
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
        // Excluding bools would split a run wherever a flag sits.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 2)] a: u8,
            #[deku(bits = 1)] flag: bool,
            #[deku(bits = 5)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![2, 1, 5])]);

        // Without `bits` a bool is a byte, as `impls::bool` reads it.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4)] a: u8,
            flag: bool,
            b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 8, 8])]);
    }

    #[test]
    fn the_rtp_header_batches_into_one_read() {
        // RFC 3550 fixed header: 9 fields, three of them flags.
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
        // The first eight sum to 64 bits; `ssrc` cannot fit. Two reads, not seven.
        assert_eq!(plan(src), vec![(0, vec![2, 1, 1, 4, 1, 7, 16, 32])]);
    }

    #[test]
    fn only_unsigned_primitives_and_bool_qualify() {
        // A packed `u8` either side, so only the type under test can break the run.
        for ty in ["i8", "i16", "f32", "MyEnum", "Vec<u8>", "[u8; 2]"] {
            let src = format!(
                r#"#[deku(endian = "big")] struct Test {{
                #[deku(bits = 4)] a: u8,
                b: {ty},
                #[deku(bits = 4)] c: u8,
            }}"#
            );
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

    /// Every attribute that must keep a field out of a run. Two fields that would
    /// otherwise batch, with the attribute on the first.
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
        // `b` is packed, so the run would form if `a` still qualified. `a` takes no
        // `bits` of its own, since `bytes` conflicts with it.
        let src = format!(
            r#"#[deku(endian = "big")] struct Test {{
            #[deku({attr})] a: u8,
            #[deku(bits = 4)] b: u8,
        }}"#
        );
        assert_eq!(
            plan(&src),
            vec![],
            "`{attr}` must keep the field out of a run"
        );
        // The same two fields batch without it, so the attribute is what stopped it.
        let src = r#"#[deku(endian = "big")] struct Test {
            a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 4])]);
    }

    #[test]
    fn the_id_storage_field_is_never_part_of_a_run() {
        // The id has already been read, so it cannot join the run behind it.
        let src = &be_struct(&[2, 3, 3]);
        assert_eq!(plan_with_id(src, false), vec![(0, vec![2, 3, 3])]);
        assert_eq!(plan_with_id(src, true), vec![(1, vec![3, 3])]);

        // One field left behind the id is no run.
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
        // `update` feeds only `DekuUpdate`, so it cannot change the read or write.
        let src = r#"#[deku(endian = "big")] struct Test {
            #[deku(bits = 4, update = "0")] a: u8,
            #[deku(bits = 4)] b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);
    }

    #[test]
    fn the_emitted_read_makes_one_call_for_the_whole_run() {
        // What round-trip tests cannot show: three fields, one call.
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

    /// `whole_width` marks a field that takes its type's full width, which is the
    /// one signal the packed test reads.
    #[test]
    fn whole_width_marks_a_field_that_fills_its_type() {
        let src = r#"#[deku(endian = "big")]
        struct Test {
            #[deku(bits = 4)]
            narrowed: u8,
            plain: u8,
            #[deku(bits = 8)]
            spelled_out: u8,
            #[deku(bits = 15)]
            narrowed_wide: u16,
            wide: u16,
            #[deku(bits = 1)] flag: bool,
            byte_flag: bool,
        }"#;
        let data = DekuData::from_input(src.parse().unwrap()).unwrap();
        let fields = data.data.as_ref().take_struct().unwrap();
        let whole: Vec<bool> = fields
            .fields
            .iter()
            .map(|f| {
                run_field(&data, f)
                    .expect("every field should qualify for a run")
                    .whole_width
            })
            .collect();
        // `bits` equal to the width is whole, as is its absence. A narrowed field
        // is not, and a bool is a byte unless `bits` narrows it.
        assert_eq!(
            whole,
            vec![false, true, true, false, true, false, true],
            "narrowed, plain, spelled_out, narrowed_wide, wide, flag, byte_flag"
        );
    }

    /// The regression #677 introduced: a struct of plain bytes holds no bit field,
    /// so sending it through the bit reader costs a shift and a mask per field and
    /// buys nothing. Such a run must stay on the byte path.
    #[test]
    fn a_run_of_only_whole_width_fields_does_not_batch() {
        // `vec![]` alone would also hold if a field were ineligible for some other
        // reason, so check that every field qualifies and is whole. Then the
        // whole-width rule is the only thing left that can reject the run.
        fn every_field_is_whole_and_eligible(src: &str) {
            let data = DekuData::from_input(src.parse().unwrap()).expect("input should parse");
            let fields = data.data.as_ref().take_struct().unwrap();
            for (i, f) in fields.fields.iter().enumerate() {
                let field = run_field(&data, f).unwrap_or_else(|| {
                    panic!("field {i} should qualify, leaving only the whole-width rule")
                });
                assert!(field.whole_width, "field {i} should be whole-width");
            }
            assert_eq!(plan(src), vec![], "a whole-width run must not batch");
        }

        // Plain `u8` fields, the shape a byte-aligned header takes.
        every_field_is_whole_and_eligible(
            r#"#[deku(endian = "big")] struct Test { a: u8, b: u8, c: u8, d: u8 }"#,
        );

        // Mixed widths, still every field whole.
        every_field_is_whole_and_eligible(
            r#"#[deku(endian = "big")] struct Test { a: u8, b: u16, c: u32 }"#,
        );

        // `bits` that merely restates the width is still whole.
        every_field_is_whole_and_eligible(
            r#"#[deku(endian = "big")]
            struct Test {
                #[deku(bits = 8)]
                a: u8,
                #[deku(bits = 16)]
                b: u16,
            }"#,
        );

        // Byte-wide bools included, since `impls::bool` reads one as a byte.
        every_field_is_whole_and_eligible(
            r#"#[deku(endian = "big")] struct Test { a: bool, b: bool, c: u8 }"#,
        );

        // No emitted call either, so the byte path really serves them.
        let data = DekuData::from_input(
            r#"#[deku(endian = "big")] struct Test { a: u8, b: u8, c: u8 }"#
                .parse()
                .unwrap(),
        )
        .unwrap();
        let emitted = emit_deku_read(&data).unwrap().to_string();
        assert_eq!(emitted.matches("read_bits_uint_msb0").count(), 0);
    }

    /// One packed field is enough to earn the batch, wherever it sits, since the
    /// whole run then needs the bit reader anyway.
    #[test]
    fn one_packed_field_makes_a_whole_width_run_batch() {
        // At the front, in the middle, and at the back. Each run carries the flag
        // per field, which is what the emitters read, so assert those too.
        let src = r#"#[deku(endian = "big")]
        struct Test {
            #[deku(bits = 4)]
            p: u8,
            a: u8,
            b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 8, 8])]);
        assert_eq!(plan_whole_width(src), vec![(0, vec![false, true, true])]);

        let src = r#"#[deku(endian = "big")]
        struct Test {
            a: u8,
            #[deku(bits = 4)]
            p: u8,
            b: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 4, 8])]);
        assert_eq!(plan_whole_width(src), vec![(0, vec![true, false, true])]);

        let src = r#"#[deku(endian = "big")]
        struct Test {
            a: u8,
            b: u8,
            #[deku(bits = 4)]
            p: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![8, 8, 4])]);
        assert_eq!(plan_whole_width(src), vec![(0, vec![true, true, false])]);

        // A one-bit bool is packed too, so it earns the batch on its own.
        let src = r#"#[deku(endian = "big")]
        struct Test {
            #[deku(bits = 1)]
            flag: bool,
            a: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![1, 8])]);
        assert_eq!(plan_whole_width(src), vec![(0, vec![false, true])]);
    }

    /// The packed test applies per run, not per struct, so a whole-width stretch
    /// beside a packed one keeps the byte path.
    #[test]
    fn a_whole_width_stretch_beside_a_packed_run_stays_unbatched() {
        // The little-endian field splits the struct in two. Only the packed half
        // batches; the plain half keeps a read per field.
        let src = r#"#[deku(endian = "big")]
        struct Test {
            a: u8,
            b: u8,
            #[deku(endian = "little")]
            split: u16,
            #[deku(bits = 4)]
            c: u8,
            #[deku(bits = 4)]
            d: u8,
        }"#;
        assert_eq!(plan(src), vec![(3, vec![4, 4])]);
        // `a` and `b` are eligible and whole, so only the rule kept them out.
        let data = DekuData::from_input(src.parse().unwrap()).unwrap();
        let fields = data.data.as_ref().take_struct().unwrap();
        for i in 0..2 {
            assert!(
                run_field(&data, fields.fields[i])
                    .expect("the plain half should still be eligible")
                    .whole_width
            );
        }
        assert_eq!(plan_whole_width(src), vec![(3, vec![false, false])]);

        // And the other way round, so the order does not matter.
        let src = r#"#[deku(endian = "big")]
        struct Test {
            #[deku(bits = 4)]
            a: u8,
            #[deku(bits = 4)]
            b: u8,
            #[deku(endian = "little")]
            split: u16,
            c: u8,
            d: u8,
        }"#;
        assert_eq!(plan(src), vec![(0, vec![4, 4])]);
        assert_eq!(plan_whole_width(src), vec![(0, vec![false, false])]);
    }

    /// Rejecting a whole-width run advances one field, so the planner retries from
    /// the second and can still reach a packed field the 64-bit cap had cut off.
    #[test]
    fn a_rejected_whole_width_run_retries_from_its_second_field() {
        // 32 + 32 fills the cap, so `p` falls outside the first run, which is then
        // whole-width and rejected. From `b` the cap leaves room for `p`, and that
        // run is packed, so `a` alone keeps the byte path.
        let src = r#"#[deku(endian = "big")]
         struct Test {
            a: u32,
            b: u32,
            #[deku(bits = 4)] p: u8,
        }"#;
        assert_eq!(plan(src), vec![(1, vec![32, 4])]);
        // The run that survives is the mixed one: a whole `u32` and packed `p`.
        assert_eq!(plan_whole_width(src), vec![(1, vec![true, false])]);

        // A third `u32` pushes the packed run one field further along: the retry
        // walks one field at a time until `p` fits, so `a` and `b` are left behind.
        let src = r#"#[deku(endian = "big")]
        struct Test {
            a: u32,
            b: u32,
            c: u32,
            #[deku(bits = 4)]
            p: u8,
        }"#;
        assert_eq!(plan(src), vec![(2, vec![32, 4])]);
        assert_eq!(plan_whole_width(src), vec![(2, vec![true, false])]);
    }

    /// A variant's first field can hold the id an `id_pat` matched, which the enum
    /// has already read. The whole-width rule judges what is left, so the first
    /// field must not count towards the packed test.
    ///
    /// Only an enum variant reaches this: `emit_field_reads` passes `use_id: false`
    /// for every struct.
    #[test]
    fn the_id_storage_field_is_left_out_of_the_packed_test() {
        // The id storage takes no attributes of its own, so it is always whole-width.
        // Behind it sit two more whole-width fields, and the run must not form.
        let src = r#"
        #[deku(id_type = "u8", endian = "big")]
        enum Test {
            #[deku(id_pat = "_")]
            CatchAll {
                captured_id: u8,
                a: u8,
                b: u8,
            },
        }"#;
        assert_eq!(plan_variant(src, 0, true), vec![]);
        // Nor without the id: every field fills its type either way.
        assert_eq!(plan_variant(src, 0, false), vec![]);

        // A packed field of the variant's own earns the batch behind the id. The id
        // is left out, so the run starts at field 1 and is two fields wide, not the
        // three it would cover if the id counted.
        let src = r#"
        #[deku(id_type = "u8", endian = "big")]
        enum Test {
            #[deku(id_pat = "_")]
            CatchAll {
                captured_id: u8,
                #[deku(bits = 4)] a: u8,
                b: u8,
            },
        }"#;
        assert_eq!(plan_variant(src, 0, true), vec![(1, vec![4, 8])]);
        // Without the id the same fields batch from 0, which is the contrast.
        assert_eq!(plan_variant(src, 0, false), vec![(0, vec![8, 4, 8])]);
    }
}
