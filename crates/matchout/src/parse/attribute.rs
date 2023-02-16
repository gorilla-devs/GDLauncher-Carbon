//! Parsing code for #[extract] options

use itertools::Itertools;
use proc_macro2::{Group, Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    parse::ParseStream, punctuated::Pair, spanned::Spanned, Arm, Expr, ExprCall, ExprPath,
    ExprStruct, Field, FieldValue, Fields, FieldsNamed, FieldsUnnamed, Member, Pat, PatBox,
    PatIdent, PatPath, PatStruct, PatTuple, PatTupleStruct, Path, PathSegment,
};

#[derive(Debug)]
pub struct ExtractPattern {
    pub target: Path,
    pub pattern: Pat,
    // coerce the matched pattern using Into::into
    pub into: bool,
}

impl ExtractPattern {
    pub fn create_arm(&self, variant: Ident, fields: &Fields) -> Arm {
        let path = ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: Path {
                leading_colon: None,
                segments: {
                    let segments = [
                        PathSegment::from(Ident::new("Self", Span::call_site())),
                        PathSegment::from(variant),
                    ];

                    segments.into_iter().collect()
                },
            },
        };

        let call_into = |path| {
            Expr::Call(ExprCall {
                attrs: Vec::new(),
                func: Box::new(Expr::Path(ExprPath {
                    attrs: Vec::new(),
                    qself: None,
                    path: Path {
                        leading_colon: Some(Default::default()),
                        segments: {
                            let segments = ["core", "convert", "Into", "into"];

                            segments
                                .map(|s| PathSegment::from(Ident::new(s, Span::call_site())))
                                .into_iter()
                                .collect()
                        },
                    },
                })),
                paren_token: Default::default(),
                args: {
                    let args = [Expr::Path(path)];

                    args.into_iter().collect()
                },
            })
        };

        Arm {
            attrs: Vec::new(),
            pat: self.pattern.clone(),
            guard: None,
            fat_arrow_token: Default::default(),
            comma: Some(Default::default()),
            body: Box::new(match fields {
                Fields::Unit => Expr::Path(path),
                Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => Expr::Call(ExprCall {
                    attrs: Vec::new(),
                    func: Box::new(Expr::Path(path)),
                    paren_token: Default::default(),
                    args: {
                        let fields = (0..unnamed.len()).map(|i| {
                            let path = ExprPath {
                                attrs: Vec::new(),
                                qself: None,
                                path: Path::from(Ident::new(&format!("_{i}"), Span::call_site())),
                            };

                            // currently `into` can only be true for single arguments,
                            // so types without `Into` impls are not an issue.
                            match self.into {
                                false => Expr::Path(path),
                                true => call_into(path),
                            }
                        });

                        fields.collect()
                    },
                }),
                Fields::Named(FieldsNamed { named, .. }) => Expr::Struct(ExprStruct {
                    attrs: Vec::new(),
                    path: path.path,
                    brace_token: Default::default(),
                    dot2_token: None,
                    rest: None,
                    fields: {
                        let fields = named.iter().map(|Field { ident, .. }| {
                            let ident = ident.clone().unwrap(); // definitely named

                            let path = ExprPath {
                                attrs: Vec::new(),
                                qself: None,
                                path: Path::from(ident.clone()),
                            };

                            FieldValue {
                                attrs: Vec::new(),
                                member: Member::Named(ident.clone()),
                                colon_token: None,
                                expr: match self.into {
                                    false => Expr::Path(path),
                                    true => call_into(path),
                                },
                            }
                        });

                        fields.collect()
                    },
                }),
            }),
        }
    }

    pub fn parse(input: ParseStream, field: Option<&str>, into: bool) -> syn::Result<Self> {
        let reparsed = reparse_tokens(input.parse::<TokenStream>()?, field);
        let mut pat = syn::parse2::<Pat>(reparsed)?;

        // paths (`Foo::Bar`) are shorthand for a single field tuple enum (`Foo::Bar(self)`)
        if let (Some(field), Pat::Path(path)) = (&field, &pat) {
            pat = Pat::TupleStruct(PatTupleStruct {
                attrs: Vec::new(),
                path: path.path.clone(),
                pat: PatTuple {
                    attrs: Vec::new(),
                    paren_token: Default::default(),
                    elems: {
                        let tuple_args = [Pat::Path(PatPath {
                            attrs: Vec::new(),
                            qself: None,
                            path: Path::from(Ident::new(field, path.span())),
                        })];

                        tuple_args.into_iter().collect()
                    },
                },
            });
        }

        let target = resolve_target(&pat)?;

        Ok(Self {
            target,
            into,
            pattern: pat,
        })
    }
}

fn resolve_target(pat: &Pat) -> syn::Result<Path> {
    match pat {
        Pat::Path(PatPath { path, .. })
        | Pat::Struct(PatStruct { path, .. })
        | Pat::TupleStruct(PatTupleStruct { path, .. }) => {
            let mut path = path.clone();
            if path.segments.len() > 1 {
                // pop last segment and the one before, then add the second segment
                // without trailing punct
                path.segments.pop();
                if let Some(Pair::Punctuated(s, _) | Pair::End(s)) = path.segments.pop() {
                    path.segments.push(s);
                }
            }

            Ok(path)
        }
        Pat::Ident(PatIdent { ident, subpat, .. }) => match subpat {
            None => Ok(Path::from(ident.clone())),
            Some(subpat) => resolve_target(&subpat.1),
        },
        Pat::Box(PatBox { pat, .. }) => resolve_target(pat),
        pat => Err(syn::Error::new_spanned(
            pat,
            "could not resolve target type from pattern",
        )),
    }
}

fn reparse_tokens(stream: TokenStream, field: Option<&str>) -> TokenStream {
    let mut collector = TokenStream::new();
    let mut iter = stream.into_iter().multipeek();

    'next: while let Some(next) = iter.next() {
        match next {
            TokenTree::Group(group) => {
                let mut new = Group::new(group.delimiter(), reparse_tokens(group.stream(), field));

                new.set_span(group.span());
                collector.append(new);
            }
            TokenTree::Ident(ident) => {
                'special: {
                    if ident == "self" {
                        let dot = match iter.peek() {
                            Some(TokenTree::Punct(dot)) if dot.as_char() == '.' => dot,
                            _ => {
                                if let Some(field) = field {
                                    collector.append(Ident::new(field, ident.span()));
                                    continue 'next;
                                } else {
                                    break 'special;
                                }
                            }
                        };

                        // span tokens
                        let mut ident_tokens = TokenStream::new();
                        ident.to_tokens(&mut ident_tokens);
                        dot.to_tokens(&mut ident_tokens);

                        let (ref next, next_tok) = match iter.peek() {
                            Some(tok @ TokenTree::Ident(x)) => (x.to_string(), tok),
                            Some(tok @ TokenTree::Literal(x)) => {
                                let string = x.to_string();
                                match string.parse::<u8>() {
                                    Ok(_) => (format!("_{string}"), tok),
                                    Err(_) => break 'special,
                                }
                            }
                            _ => break 'special,
                        };

                        next_tok.to_tokens(&mut ident_tokens);
                        collector.append(Ident::new(next, ident_tokens.span()));

                        // skip tokens
                        let _ = iter.next();
                        let _ = iter.next();

                        continue 'next;
                    }
                }

                collector.append(ident.clone());
            }
            token => collector.append(token),
        };
    }

    collector
}
