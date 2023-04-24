use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, FieldsNamed, Ident, Variant,
};

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

    let gen = quote! {
        #(#attrs)*
        pub struct #struct_name {
            #fields
        }

        impl #struct_name {
            pub fn into_query_parameters(&self) -> Result<String, serde_urlencoded::ser::Error> {
                serde_urlencoded::to_string(self)
            }
        }
    };

    TokenStream::from(gen)
}

#[proc_macro_derive(FromTo, attributes(to))]
pub fn from_to(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident_from = &input.ident;
    let ident_to = input
        .attrs
        .iter()
        .find(|attr| attr.path().get_ident().map_or(false, |ident| ident == "to"))
        .and_then(|attr| attr.parse_args::<syn::Path>().ok())
        .expect("Missing #[to(...)] attribute or invalid syntax");

    match &input.data {
        syn::Data::Struct(_) => from_to_struct(&input, ident_from, &ident_to),
        syn::Data::Enum(_) => from_to_enum(&input, ident_from, &ident_to),
        _ => panic!("FromTo macro only supports structs and enums"),
    }
}

fn from_to_struct(input: &DeriveInput, ident_from: &Ident, ident_to: &syn::Path) -> TokenStream {
    let field_assignments_from = generate_field_assignments(input);
    let field_assignments_to = generate_field_assignments(input);

    let expanded = quote! {
        impl From<#ident_from> for #ident_to {
            fn from(src: #ident_from) -> Self {
                Self {
                    #( #field_assignments_from ),*
                }
            }
        }

        impl From<#ident_to> for #ident_from {
            fn from(src: #ident_to) -> Self {
                Self {
                    #( #field_assignments_to ),*
                }
            }
        }
    };

    expanded.into()
}

fn from_to_enum(input: &DeriveInput, ident_from: &Ident, ident_to: &syn::Path) -> TokenStream {
    let variants = if let syn::Data::Enum(data_enum) = &input.data {
        &data_enum.variants
    } else {
        panic!("Expected an enum");
    };

    let from_match_arms = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        quote! { #ident_from::#variant_ident => #ident_to::#variant_ident }
    });

    let to_match_arms = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        quote! { #ident_to::#variant_ident => #ident_from::#variant_ident }
    });

    let expanded = quote! {
        impl From<#ident_from> for #ident_to {
            fn from(src: #ident_from) -> Self {
                match src {
                    #( #from_match_arms, )*
                }
            }
        }

        impl From<#ident_to> for #ident_from {
            fn from(src: #ident_to) -> Self {
                match src {
                    #( #to_match_arms, )*
                }
            }
        }
    };

    expanded.into()
}

fn generate_field_assignments(input: &syn::DeriveInput) -> Vec<proc_macro2::TokenStream> {
    let data_struct = match &input.data {
        syn::Data::Struct(data_struct) => data_struct,
        _ => panic!("Named fields required"),
    };

    let fields_named = match &data_struct.fields {
        Fields::Named(fields_named) => &fields_named.named,
        _ => panic!("Named fields required"),
    };

    fields_named
        .iter()
        .map(|field| {
            let field_name = &field.ident;
            if let syn::Type::Path(type_path) = &field.ty {
                let segment = type_path.path.segments.last().unwrap();
                let inner_type = if segment.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            Some(inner_type)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(_) = inner_type {
                    quote! { #field_name: src.#field_name.map(|v| v.into()) }
                } else {
                    quote! { #field_name: src.#field_name.into() }
                }
            } else {
                quote! { #field_name: src.#field_name.into() }
            }
        })
        .collect()
}
