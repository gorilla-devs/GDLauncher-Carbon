use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derives name-based row mapping and `COLUMNS` schema metadata.
///
/// Each field is read by column name (never positional index). Column names
/// default to the field name mapped snake_case → camelCase, matching how PCR
/// mapped the schema; `#[column("explicitName")]` overrides the default.
/// `DateTime<FixedOffset>` fields route through `carbon_repos::dbtypes::DbDateTime`.
///
/// `#[nullable(true|false)]` explicitly declares a column's nullability instead
/// of inferring it from the field being `Option<T>`. SQL expression / aggregate
/// columns (`(x IS NOT NULL) AS flag`, `COUNT(*)`, …) have no resolvable origin
/// column, so the origin-based nullability lint requires them to either be
/// `Option` or carry this attribute.
#[proc_macro_derive(FromRow, attributes(column, nullable))]
pub fn derive_from_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(s) => &s.fields,
        _ => panic!("FromRow only supports structs"),
    };

    let mut getters = Vec::new();
    let mut specs = Vec::new();
    for f in fields {
        let ident = f.ident.as_ref().expect("named fields only");
        let col = column_name(f, ident);
        let (ty_class, type_nullable, inner) = classify(&f.ty);
        // Row reads follow the field's Rust type (Option<T> vs T); the
        // `#[nullable(...)]` override affects only the reported ColumnSpec.
        let getter = if is_datetime(&inner) {
            if type_nullable {
                quote! { #ident: row.get::<_, Option<carbon_repos::dbtypes::DbDateTime>>(#col)?.map(|d| d.0) }
            } else {
                quote! { #ident: row.get::<_, carbon_repos::dbtypes::DbDateTime>(#col)?.0 }
            }
        } else {
            quote! { #ident: row.get(#col)? }
        };
        getters.push(getter);
        let (nullable, explicit) = match nullable_override(f) {
            Some(b) => (b, true),
            None => (type_nullable, false),
        };
        specs.push(quote! {
            carbon_repos::from_row::ColumnSpec { name: #col, ty: carbon_repos::from_row::TypeClass::#ty_class, nullable: #nullable, explicit_nullable: #explicit }
        });
    }

    quote! {
        impl carbon_repos::from_row::FromRow for #name {
            const COLUMNS: &'static [carbon_repos::from_row::ColumnSpec] = &[ #(#specs),* ];
            fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
                Ok(Self { #(#getters),* })
            }
        }
    }
    .into()
}

/// Explicit nullability from `#[nullable(true|false)]`, if present.
fn nullable_override(field: &syn::Field) -> Option<bool> {
    for attr in &field.attrs {
        if attr.path().is_ident("nullable") {
            let lit: syn::LitBool = attr
                .parse_args()
                .expect("#[nullable(true|false)] expects a bool literal");
            return Some(lit.value());
        }
    }
    None
}

/// Column name for a field: `#[column("...")]` override, else snake→camel.
fn column_name(field: &syn::Field, ident: &syn::Ident) -> String {
    for attr in &field.attrs {
        if attr.path().is_ident("column") {
            let lit: syn::LitStr = attr
                .parse_args()
                .expect("#[column(\"...\")] expects a string literal");
            return lit.value();
        }
    }
    snake_to_camel(&ident.to_string())
}

/// snake_case → camelCase: first segment unchanged, capitalize the rest.
fn snake_to_camel(s: &str) -> String {
    let mut out = String::new();
    for (i, part) in s.split('_').enumerate() {
        if i == 0 {
            out.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        }
    }
    out
}

/// Returns `(TypeClass variant ident, nullable, inner type)`.
/// `Option<T>` is nullable with T's class; everything else is non-nullable.
fn classify(ty: &syn::Type) -> (syn::Ident, bool, syn::Type) {
    if let Some(inner) = option_inner(ty) {
        let variant = class_of(inner);
        (variant, true, inner.clone())
    } else {
        let variant = class_of(ty);
        (variant, false, ty.clone())
    }
}

/// Maps a concrete (non-Option) type to its `TypeClass` variant ident.
///
/// `Vec` maps to `Blob` only for `Vec<u8>`; any other element type is a
/// compile error (a blob column is raw bytes — `Vec<i32>` etc. never round-trip
/// through a SQLite BLOB and would silently mis-decode).
fn class_of(ty: &syn::Type) -> syn::Ident {
    let seg = last_segment(ty)
        .unwrap_or_else(|| panic!("FromRow: unsupported field type (expected a named type)"));
    let variant = match seg.ident.to_string().as_str() {
        "String" => "Text",
        "i32" | "i64" => "Integer",
        "f64" => "Real",
        "bool" => "Bool",
        "Vec" => {
            if !is_vec_u8(seg) {
                panic!("FromRow: blob columns must be `Vec<u8>`; other `Vec<T>` element types are unsupported");
            }
            "Blob"
        }
        "DateTime" => "DateTime",
        other => panic!("FromRow: unsupported field type `{}`", other),
    };
    syn::Ident::new(variant, seg.ident.span())
}

/// True when `seg` is `Vec<u8>` (element type's last path segment is `u8`).
fn is_vec_u8(seg: &syn::PathSegment) -> bool {
    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
            return last_segment(inner).map(|s| s.ident == "u8").unwrap_or(false);
        }
    }
    false
}

/// Inner type of `Option<T>`, if `ty` is an `Option`.
fn option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Last path segment of a `Type::Path`.
fn last_segment(ty: &syn::Type) -> Option<&syn::PathSegment> {
    if let syn::Type::Path(tp) = ty {
        tp.path.segments.last()
    } else {
        None
    }
}

/// Last path segment ident of a `Type::Path`.
fn last_segment_ident(ty: &syn::Type) -> Option<&syn::Ident> {
    last_segment(ty).map(|s| &s.ident)
}

/// True when the type's last path segment is `DateTime`.
fn is_datetime(ty: &syn::Type) -> bool {
    last_segment_ident(ty).map(|i| i == "DateTime").unwrap_or(false)
}

#[proc_macro_attribute]
pub fn into_query_parameters(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = input.ident;
    let fields = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields_named) => fields_named.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("This macro can only be used on structs"),
    };

    let attrs = &input.attrs;

    let generated = quote! {
        #(#attrs)*
        pub struct #struct_name {
            #fields
        }

        impl #struct_name {
            pub fn into_query_parameters(&self) -> Result<String, serde_qs::Error> {
                serde_qs::to_string(self)
            }
        }
    };

    TokenStream::from(generated)
}
