use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{
    Data, DeriveInput, Expr, ExprLit, Field, FieldsNamed, Fields, Lit, LitInt, LitStr, Type,
    TypePath, parse_macro_input,
};

#[proc_macro_derive(UciOptions, attributes(uci))]
pub fn derive_uci_options(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[derive(Clone, Copy)]
enum KindTag {
    Check,
    Spin,
    Combo,
    Button,
    StringOption,
}

enum ResolvedKind {
    Check { default: bool },
    Spin { default: i64, min: i64, max: i64 },
    Combo { default: String, variants: Vec<String> },
    Button,
    StringOption { default: String },
}

struct OptionField {
    ident: syn::Ident,
    name: String,
    kind: ResolvedKind,
}

#[derive(Default)]
struct RawAttributes {
    name: Option<String>,
    min: Option<i64>,
    max: Option<i64>,
    default: Option<Lit>,
    variants: Option<Vec<String>>,
    kind: Option<KindTag>,
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let fields = named_fields(&input)?;
    let parsed = fields
        .named
        .iter()
        .map(parse_field)
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(implement(&input, &parsed))
}

fn named_fields(input: &DeriveInput) -> syn::Result<&FieldsNamed> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => Ok(named),
            _ => Err(syn::Error::new_spanned(input, "UciOptions requires named fields")),
        },
        _ => Err(syn::Error::new_spanned(
            input,
            "UciOptions can only be derived for structs",
        )),
    }
}

fn parse_field(field: &Field) -> syn::Result<OptionField> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new_spanned(field, "expected a named field"))?;
    let mut raw = RawAttributes::default();
    for attribute in &field.attrs {
        if attribute.path().is_ident("uci") {
            attribute.parse_nested_meta(|meta| collect_attribute(&meta, &mut raw))?;
        }
    }
    let name = raw
        .name
        .clone()
        .unwrap_or_else(|| pascal_case(&ident.to_string()));
    let kind = resolve_kind(field, &raw)?;
    Ok(OptionField { ident, name, kind })
}

fn collect_attribute(meta: &ParseNestedMeta, raw: &mut RawAttributes) -> syn::Result<()> {
    let key = meta.path.require_ident()?.to_string();
    match key.as_str() {
        "name" => raw.name = Some(parse_string(meta)?),
        "min" => raw.min = Some(parse_int(meta)?),
        "max" => raw.max = Some(parse_int(meta)?),
        "default" => raw.default = Some(meta.value()?.parse()?),
        "variants" => raw.variants = Some(parse_variants(meta)?),
        "check" => raw.kind = Some(KindTag::Check),
        "spin" => raw.kind = Some(KindTag::Spin),
        "combo" => raw.kind = Some(KindTag::Combo),
        "button" => raw.kind = Some(KindTag::Button),
        "string" => raw.kind = Some(KindTag::StringOption),
        other => return Err(meta.error(format!("unknown uci attribute `{other}`"))),
    }
    Ok(())
}

fn parse_string(meta: &ParseNestedMeta) -> syn::Result<String> {
    let literal: LitStr = meta.value()?.parse()?;
    Ok(literal.value())
}

fn parse_int(meta: &ParseNestedMeta) -> syn::Result<i64> {
    let literal: LitInt = meta.value()?.parse()?;
    literal.base10_parse()
}

fn parse_variants(meta: &ParseNestedMeta) -> syn::Result<Vec<String>> {
    match meta.value()?.parse()? {
        Expr::Array(array) => array.elems.iter().map(expression_string).collect(),
        other => Err(syn::Error::new_spanned(other, "expected an array of string literals")),
    }
}

fn expression_string(expression: &Expr) -> syn::Result<String> {
    match expression {
        Expr::Lit(ExprLit { lit: Lit::Str(literal), .. }) => Ok(literal.value()),
        _ => Err(syn::Error::new_spanned(expression, "expected a string literal")),
    }
}

fn resolve_kind(field: &Field, raw: &RawAttributes) -> syn::Result<ResolvedKind> {
    let tag = match raw.kind {
        Some(tag) => tag,
        None => infer_kind(&field.ty)
            .ok_or_else(|| syn::Error::new_spanned(&field.ty, "cannot infer uci option kind"))?,
    };
    match tag {
        KindTag::Check => Ok(ResolvedKind::Check {
            default: default_bool(raw),
        }),
        KindTag::Spin => Ok(ResolvedKind::Spin {
            default: default_int(field, raw)?,
            min: require_int(field, raw.min, "min")?,
            max: require_int(field, raw.max, "max")?,
        }),
        KindTag::Combo => Ok(ResolvedKind::Combo {
            default: default_string(field, raw)?,
            variants: require_variants(field, raw)?,
        }),
        KindTag::Button => Ok(ResolvedKind::Button),
        KindTag::StringOption => Ok(ResolvedKind::StringOption {
            default: raw
                .default
                .as_ref()
                .map(string_literal)
                .transpose()?
                .unwrap_or_default(),
        }),
    }
}

fn infer_kind(ty: &Type) -> Option<KindTag> {
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Some(KindTag::Button),
        Type::Path(path) => infer_from_path(path),
        _ => None,
    }
}

fn infer_from_path(path: &TypePath) -> Option<KindTag> {
    match path.path.segments.last()?.ident.to_string().as_str() {
        "bool" => Some(KindTag::Check),
        "String" => Some(KindTag::StringOption),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            Some(KindTag::Spin)
        }
        _ => None,
    }
}

fn default_bool(raw: &RawAttributes) -> bool {
    matches!(raw.default, Some(Lit::Bool(ref literal)) if literal.value)
}

fn default_int(field: &Field, raw: &RawAttributes) -> syn::Result<i64> {
    match &raw.default {
        Some(Lit::Int(literal)) => literal.base10_parse(),
        _ => Err(syn::Error::new_spanned(field, "spin option requires `default`")),
    }
}

fn default_string(field: &Field, raw: &RawAttributes) -> syn::Result<String> {
    match &raw.default {
        Some(Lit::Str(literal)) => Ok(literal.value()),
        _ => Err(syn::Error::new_spanned(field, "combo option requires a string `default`")),
    }
}

fn string_literal(literal: &Lit) -> syn::Result<String> {
    match literal {
        Lit::Str(literal) => Ok(literal.value()),
        _ => Err(syn::Error::new_spanned(literal, "expected a string literal")),
    }
}

fn require_int(field: &Field, value: Option<i64>, label: &str) -> syn::Result<i64> {
    value.ok_or_else(|| syn::Error::new_spanned(field, format!("spin option requires `{label}`")))
}

fn require_variants(field: &Field, raw: &RawAttributes) -> syn::Result<Vec<String>> {
    raw.variants
        .clone()
        .ok_or_else(|| syn::Error::new_spanned(field, "combo option requires `variants`"))
}

fn implement(input: &DeriveInput, fields: &[OptionField]) -> TokenStream2 {
    let name = &input.ident;
    let (implementation, type_generics, where_clause) = input.generics.split_for_impl();
    let declarations = fields.iter().map(declaration);
    let arms = fields.iter().map(set_arm);
    quote! {
        impl #implementation ::aspen_uci::UciOptions for #name #type_generics #where_clause {
            fn declarations() -> ::std::vec::Vec<::aspen_uci::UciOptionDeclaration> {
                ::std::vec![ #(#declarations),* ]
            }

            fn set(
                &mut self,
                name: &str,
                value: ::core::option::Option<&str>,
            ) -> ::core::result::Result<(), ::aspen_uci::UciParseError> {
                match name {
                    #(#arms)*
                    other => ::core::result::Result::Err(
                        ::aspen_uci::UciParseError::UnknownOption(other.to_owned()),
                    ),
                }
            }
        }
    }
}

fn declaration(field: &OptionField) -> TokenStream2 {
    let name = &field.name;
    let kind = kind_tokens(&field.kind);
    quote! {
        ::aspen_uci::UciOptionDeclaration { name: #name, kind: #kind }
    }
}

fn kind_tokens(kind: &ResolvedKind) -> TokenStream2 {
    match kind {
        ResolvedKind::Check { default } => {
            quote! { ::aspen_uci::UciOptionKind::Check { default: #default } }
        }
        ResolvedKind::Spin { default, min, max } => {
            quote! { ::aspen_uci::UciOptionKind::Spin { default: #default, min: #min, max: #max } }
        }
        ResolvedKind::Combo { default, variants } => quote! {
            ::aspen_uci::UciOptionKind::Combo {
                default: #default.to_owned(),
                variants: ::std::vec![ #(#variants.to_owned()),* ],
            }
        },
        ResolvedKind::Button => quote! { ::aspen_uci::UciOptionKind::Button },
        ResolvedKind::StringOption { default } => {
            quote! { ::aspen_uci::UciOptionKind::String { default: #default.to_owned() } }
        }
    }
}

fn set_arm(field: &OptionField) -> TokenStream2 {
    let name = &field.name;
    let assignment = assignment(field);
    quote! {
        #name => { #assignment ::core::result::Result::Ok(()) }
    }
}

fn assignment(field: &OptionField) -> TokenStream2 {
    let ident = &field.ident;
    let name = &field.name;
    match &field.kind {
        ResolvedKind::Check { .. } => quote! {
            let raw = value.ok_or(::aspen_uci::UciParseError::MissingArgument("value"))?;
            self.#ident = raw == "true";
        },
        ResolvedKind::Spin { .. } => quote! {
            let raw = value.ok_or(::aspen_uci::UciParseError::MissingArgument("value"))?;
            self.#ident = raw
                .parse()
                .map_err(|source| ::aspen_uci::UciParseError::InvalidInteger { field: #name, source })?;
        },
        ResolvedKind::Combo { .. } | ResolvedKind::StringOption { .. } => quote! {
            let raw = value.ok_or(::aspen_uci::UciParseError::MissingArgument("value"))?;
            self.#ident = raw.to_owned();
        },
        ResolvedKind::Button => quote! {
            let _ = value;
        },
    }
}

fn pascal_case(field_name: &str) -> String {
    field_name
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(capitalize)
        .collect()
}

fn capitalize(segment: &str) -> String {
    let mut characters = segment.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}
