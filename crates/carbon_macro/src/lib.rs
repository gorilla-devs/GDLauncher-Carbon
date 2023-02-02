extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::__private::{bool, TokenStream2};
use syn::{parse_macro_input, Field, ItemStruct, PathArguments, Type};

fn is_app_component_field(field: &&Field) -> bool {
    match field.ty {
        Type::Path(ref type_path) => {
            let mut type_string = type_path.path.to_token_stream().to_string();
            type_string.retain(|c| !c.is_whitespace());
            type_string.starts_with("AppComponentContainer<")
        }
        _ => false,
    }
}

fn expand_app_field_getter(field: &Field) -> TokenStream2 {
    let field_name = field
        .ident
        .clone()
        .expect("field without name, expected a name for the field !")
        .to_token_stream();

    let field_type = match field.ty {
        Type::Path(ref type_path) => {
            let type_last_path_segment = type_path
                .path
                .segments
                .last()
                .expect("expected generic type definition !");
            match type_last_path_segment.arguments {
                PathArguments::AngleBracketed(ref generic) => generic.args.clone(),
                _ => panic!("unable to extract generic from AppComponentContainer!"),
            }
        }
        _ => panic!("expected field with defined path as type!"),
    };

    let component_name = format!("{field_name}");
    let getter_name = format_ident!("get_{field_name}");

    quote! {
        pub async fn #getter_name(&self) -> Result<RwLockReadGuard<#field_type>, AppError> {
            Ok(self
                .#field_name
                .as_ref()
                .ok_or_else(|| AppError::ComponentIsMissing(#component_name.to_string()))?
                .read()
                .await)
        }
    }
}

fn expand_gd_launcher_app(app_struct: ItemStruct) -> TokenStream {
    let app_struct_name = app_struct.ident.clone();
    let getters: Vec<_> = app_struct
        .fields
        .iter()
        .filter(is_app_component_field)
        .map(expand_app_field_getter)
        .collect();
    let getter = getters.get(0).unwrap();

    let tokens = quote! {
        #app_struct
        impl #app_struct_name {
            #(#getters)*
        }
    };
    tokens.into()
}

/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
#[proc_macro_attribute]
pub fn gd_launcher_app(_: TokenStream, input: TokenStream) -> TokenStream {
    let app_struct = parse_macro_input!(input as ItemStruct);
    expand_gd_launcher_app(app_struct)
}

#[proc_macro_attribute]
pub fn gd_launcher_app_component(_: TokenStream, _: TokenStream) -> TokenStream {
    // fixme : to finish
    unimplemented!()
}
