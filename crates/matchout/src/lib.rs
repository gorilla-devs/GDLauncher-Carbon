use parse::parseenum::ExtractEnum;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, Arm, ItemEnum, Path};

mod parse;

#[proc_macro_derive(Extract, attributes(extract, into))]
pub fn extract(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ExtractEnum);

    let mut tokens = TokenStream::new();
    for (target, arms) in input.matches {
        gen_match(&input.item, &target, arms).to_tokens(&mut tokens);
    }

    tokens.into()
}

fn gen_match(
    ItemEnum {
        ident, generics, ..
    }: &ItemEnum,
    target: &Path,
    arm: Vec<Arm>,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote::quote! {
        impl #impl_generics ::core::convert::From<#target> for #ident #ty_generics
        #where_clause {
            fn from(value: #target) -> Self {
                match value {
                    #(#arm)*
                }
            }
        }
    }
}
