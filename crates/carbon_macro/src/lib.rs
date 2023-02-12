extern crate proc_macro;

use proc_macro::TokenStream;
use std::fs;

use quote::quote;
use syn::__private::bool;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, ItemFn};

/// Example of user-defined [derive mode macro][1]
///
/// [1]: https://doc.rust-lang.org/reference/procedural-macros.html#derive-mode-macros
#[proc_macro_derive(GDLauncherApp)]
pub fn app_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    fn is_manager_field(field: &&Field) -> bool {
        true
    }

    let tokens = quote! {
        struct Hello;
    };

    if let Data::Struct(app_struct) = input.data {
        /*app_struct.fields.iter()
        .filter(is_manager_field)
        .map(|field| {
            field.ty
        });*/
    } else {
        panic!("")
    };

    tokens.into()
}

#[proc_macro_derive(GDLauncherAppManager)]
pub fn manager_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    fn is_manager_field(field: &&Field) -> bool {
        true
    }

    let tokens = quote! {
        struct Hello;
    };

    if let Data::Struct(app_struct) = input.data {
        /*app_struct.fields.iter()
        .filter(is_manager_field)
        .map(|field| {
            field.ty
        });*/
    } else {
        panic!("")
    };

    tokens.into()
}
