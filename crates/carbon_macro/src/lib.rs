use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

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

/// Derive macro for automatic rusqlite Row mapping.
///
/// Generates a `from_row` method that maps database columns to struct fields.
/// By default, field names are converted from snake_case to camelCase for
/// column lookup (e.g., `full_version` maps to column `fullVersion`).
///
/// # Container Attributes (uses serde attributes)
///
/// - `#[serde(rename_all = "camelCase")]` - Convert field names to camelCase (default).
/// - `#[serde(rename_all = "snake_case")]` - Keep field names as snake_case.
///
/// # Field Attributes (uses serde attributes)
///
/// - `#[serde(rename = "name")]` - Override the column name for a specific field.
///   Use this when the column name doesn't follow the rename_all convention.
///
/// # DateTime Handling
///
/// `DateTime<Utc>` and `Option<DateTime<Utc>>` fields are automatically detected
/// and parsed from string columns (SQLite stores datetimes as TEXT).
/// - Required fields default to `Utc::now()` on parse failure with a warning log.
/// - Optional fields become `None` on parse failure.
///
/// # Example
///
/// ```ignore
/// use carbon_macro::FromRow;
/// use chrono::{DateTime, Utc};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, FromRow)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Account {
///     pub uuid: String,
///     pub last_used: DateTime<Utc>,  // Auto-detected, maps to "lastUsed"
///     pub token_expires: Option<DateTime<Utc>>,  // Auto-detected, maps to "tokenExpires"
///     #[serde(rename = "status_code")]  // Override for mixed-case DB column
///     pub status_code: i32,
/// }
/// ```
///
/// This generates:
///
/// ```ignore
/// impl Account {
///     pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
///         Ok(Self {
///             uuid: row.get("uuid")?,
///             last_used: row.get::<_, String>("lastUsed")?
///                 .parse()
///                 .unwrap_or_else(|_| chrono::Utc::now()),
///             token_expires: row.get::<_, Option<String>>("tokenExpires")?
///                 .and_then(|s| s.parse().ok()),
///             status_code: row.get("status_code")?,
///         })
///     }
/// }
/// ```
#[proc_macro_derive(FromRow)]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let rename_all = get_rename_all(&input.attrs);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("FromRow only supports structs with named fields"),
        },
        _ => panic!("FromRow only supports structs"),
    };

    let field_extractions: Vec<_> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let column_name = get_serde_rename(&field.attrs).unwrap_or_else(|| {
                match rename_all {
                    RenameAll::CamelCase => to_camel_case(&field_name.to_string()),
                    RenameAll::SnakeCase => field_name.to_string(),
                }
            });
            let is_optional = is_option_type(&field.ty);
            let is_datetime = is_datetime_type(&field.ty);

            if is_datetime {
                let struct_name_str = struct_name.to_string();
                let field_name_str = field_name.to_string();
                if is_optional {
                    // Option<DateTime<Utc>> - parse with .and_then(), None on failure
                    quote! {
                        #field_name: row.get::<_, Option<String>>(#column_name)?
                            .and_then(|s| s.parse().ok())
                    }
                } else {
                    // DateTime<Utc> - parse with fallback to Utc::now(), but log warning
                    quote! {
                        #field_name: {
                            let raw_value = row.get::<_, String>(#column_name)?;
                            raw_value.parse().unwrap_or_else(|e| {
                                tracing::warn!(
                                    struct_name = #struct_name_str,
                                    field = #field_name_str,
                                    column = #column_name,
                                    raw_value = %raw_value,
                                    error = %e,
                                    "DateTime parse failed, using Utc::now() as fallback"
                                );
                                chrono::Utc::now()
                            })
                        }
                    }
                }
            } else {
                // Standard field - direct get
                quote! {
                    #field_name: row.get(#column_name)?
                }
            }
        })
        .collect();

    let expanded = quote! {
        impl #struct_name {
            /// Creates an instance from a database row.
            ///
            /// This method is auto-generated by the `FromRow` derive macro.
            pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
                Ok(Self {
                    #(#field_extractions,)*
                })
            }
        }
    };

    TokenStream::from(expanded)
}

/// Rename strategy for field-to-column name mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameAll {
    /// Convert snake_case to camelCase (default).
    CamelCase,
    /// Keep snake_case as-is.
    SnakeCase,
}

/// Extracts the `rename_all` value from `#[serde(rename_all = "...")]`.
fn get_rename_all(attrs: &[syn::Attribute]) -> RenameAll {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            // Parse the attribute arguments as MetaNameValue (e.g., rename_all = "camelCase")
            let nested = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            );

            if let Ok(nested) = nested {
                for meta in nested {
                    if let syn::Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("rename_all") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &nv.value {
                                let value = lit_str.value();
                                return match value.as_str() {
                                    "snake_case" => RenameAll::SnakeCase,
                                    "camelCase" => RenameAll::CamelCase,
                                    _ => panic!(
                                        "Unknown rename_all value: '{}'. Expected 'camelCase' or 'snake_case'",
                                        value
                                    ),
                                };
                            } else {
                                panic!("rename_all value must be a string literal");
                            }
                        }
                    }
                }
            }
        }
    }
    RenameAll::CamelCase // Default
}

/// Extracts the column name from a `#[serde(rename = "name")]` attribute.
fn get_serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            let nested = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            );

            if let Ok(nested) = nested {
                for meta in nested {
                    if let syn::Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("rename") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &nv.value {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Checks if a type is `Option<T>`.
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Checks if a type is `DateTime<...>` or `Option<DateTime<...>>`.
///
/// This enables automatic detection of DateTime fields without requiring
/// a `#[datetime]` attribute. The macro will generate string parsing code
/// for these fields since SQLite stores datetimes as TEXT.
fn is_datetime_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            // Direct DateTime<...>
            if segment.ident == "DateTime" {
                return true;
            }
            // Option<DateTime<...>> - recurse into the inner type
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return is_datetime_type(inner);
                    }
                }
            }
        }
    }
    false
}

/// Converts snake_case to camelCase.
///
/// Examples:
/// - `id` -> `id`
/// - `full_version` -> `fullVersion`
/// - `is_valid` -> `isValid`
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else if i == 0 {
            // First character stays lowercase
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("id"), "id");
        assert_eq!(to_camel_case("full_version"), "fullVersion");
        assert_eq!(to_camel_case("is_valid"), "isValid");
        assert_eq!(to_camel_case("java_type"), "javaType");
        assert_eq!(to_camel_case("release_channel"), "releaseChannel");
        assert_eq!(to_camel_case("some_long_field_name"), "someLongFieldName");
    }
}
