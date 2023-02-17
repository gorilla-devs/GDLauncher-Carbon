use std::collections::HashMap;

use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Arm, Field, Fields, FieldsNamed, FieldsUnnamed, ItemEnum, Pat, PatPath, Path, Type, TypePath,
};

use super::attribute::ExtractPattern;

#[derive(Debug)]
pub struct ExtractEnum {
    pub item: ItemEnum,
    pub matches: HashMap<Path, Vec<Arm>>,
}

impl Parse for ExtractEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item = input.parse::<ItemEnum>()?;

        let mut matches = HashMap::<Path, Vec<Arm>>::new();

        for variant in &item.variants {
            let mut has_outer_arms = false;

            for attr in &variant.attrs {
                if attr.path.is_ident("extract") {
                    let pattern = attr.parse_args_with(|input: ParseStream| {
                        ExtractPattern::parse(input, None, false)
                    })?;

                    let arm = pattern.create_arm(variant.ident.clone(), &variant.fields);
                    let arms = matches.entry(pattern.target).or_insert(Vec::new());
                    arms.push(arm);

                    has_outer_arms = true;
                }
            }

            match &variant.fields {
                Fields::Unit => {}
                Fields::Named(FieldsNamed { named: fields, .. })
                | Fields::Unnamed(FieldsUnnamed {
                    unnamed: fields, ..
                }) => {
                    let mut inline_extract = Option::None;

                    let mut iter = fields.iter();
                    if let Some(Field {
                        attrs,
                        ident: field_ident,
                        ty,
                        ..
                    }) = iter.next()
                    {
                        for attr in attrs {
                            let Some(attrtype) = attr.path.get_ident() else { continue };

                            match &attrtype.to_string() as &str {
                                "extract" => {
                                    if attr.tokens.is_empty() {
                                        if let Type::Path(TypePath { qself: None, path }) = ty {
                                            inline_extract = Some((
                                                ExtractPattern {
                                                    target: path.clone(),
                                                    pattern: Pat::Path(PatPath {
                                                        attrs: Vec::new(),
                                                        qself: None,
                                                        path: Path::from(Ident::new(
                                                            "_0",
                                                            attr.span(),
                                                        )),
                                                    }),
                                                    into: false,
                                                },
                                                attr,
                                            ));
                                        } else {
                                            return Err(syn::Error::new_spanned(attr, "#[extract] with no args can only be applied to simple paths"));
                                        }
                                    } else {
                                        let pattern =
                                            attr.parse_args_with(|input: ParseStream| {
                                                ExtractPattern::parse(
                                                    input,
                                                    Some(&match field_ident {
                                                        Some(ident) => ident.to_string(),
                                                        None => String::from("_0"),
                                                    }),
                                                    false,
                                                )
                                            })?;
                                        inline_extract = Some((pattern, attr));
                                    }
                                }
                                "into" => {
                                    let into_path = attr.parse_args::<Path>()?;

                                    inline_extract = Some((
                                        ExtractPattern {
                                            target: into_path,
                                            pattern: Pat::Path(PatPath {
                                                attrs: Vec::new(),
                                                qself: None,
                                                path: Path::from(Ident::new("_0", attr.span())),
                                            }),
                                            into: true,
                                        },
                                        attr,
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }

                    if let Some((pattern, attr)) = inline_extract {
                        if iter.next().is_some() {
                            return Err(syn::Error::new_spanned(
								attr,
								"#[extract] can only be used inline with single field enum variants"
							));
                        } else if has_outer_arms {
                            return Err(syn::Error::new_spanned(
								attr,
								"#[extract] can only be used inline when not present on the enum variant"
							));
                        }

                        let arm = pattern.create_arm(variant.ident.clone(), &variant.fields);
                        let arms = matches.entry(pattern.target).or_insert(Vec::new());
                        arms.push(arm);
                    }
                }
            }
        }

        Ok(Self { item, matches })
    }
}
