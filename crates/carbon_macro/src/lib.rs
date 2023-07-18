use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
            pub fn into_query_parameters(&self) -> Result<String, serde_qs::Error> {
                serde_qs::to_string(self)
            }
        }
    };

    TokenStream::from(gen)
}
