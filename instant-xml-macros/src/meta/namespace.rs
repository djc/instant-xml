use std::collections::BTreeMap;
use std::fmt;

use proc_macro2::{Group, Literal, Punct, TokenStream, TokenTree};
use quote::ToTokens;
use syn::punctuated::Punctuated;

pub enum Namespace {
    Path(syn::Path),
    Literal(Literal),
}

impl ToTokens for Namespace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Namespace::Path(path) => path.to_tokens(tokens),
            Namespace::Literal(lit) => lit.to_tokens(tokens),
        }
    }
}

impl fmt::Debug for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(arg0) => f
                .debug_tuple("Path")
                .field(&arg0.into_token_stream().to_string())
                .finish(),
            Self::Literal(arg0) => f.debug_tuple("Literal").field(arg0).finish(),
        }
    }
}

#[derive(Debug, Default)]
pub struct NamespaceMeta {
    pub uri: Option<Namespace>,
    pub prefixes: BTreeMap<String, Namespace>,
}

impl NamespaceMeta {
    pub fn from_tokens(group: Group) -> Self {
        let mut new = NamespaceMeta::default();
        let mut state = NsState::Start;
        for tree in group.stream() {
            state = match (state, tree) {
                (NsState::Start, TokenTree::Literal(lit)) => {
                    new.uri = Some(Namespace::Literal(lit));
                    NsState::Comma
                }
                (NsState::Start, TokenTree::Punct(punct)) if punct.as_char() == ':' => {
                    NsState::Path {
                        colon1: Some(punct),
                        colon2: None,
                        path: None,
                    }
                }
                (NsState::Start, TokenTree::Ident(id)) => NsState::Path {
                    colon1: None,
                    colon2: None,
                    path: Some(syn::Path::from(id)),
                },
                (NsState::Comma, TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                    NsState::Prefix
                }
                (
                    NsState::Path {
                        colon1: None,
                        colon2: None,
                        path,
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == ':' => NsState::Path {
                    colon1: Some(punct),
                    colon2: None,
                    path,
                },
                (
                    NsState::Path {
                        colon1: colon1 @ Some(_),
                        colon2: None,
                        path,
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == ':' => NsState::Path {
                    colon1,
                    colon2: Some(punct),
                    path,
                },
                (
                    NsState::Path {
                        colon1: Some(colon1),
                        colon2: Some(colon2),
                        path,
                    },
                    TokenTree::Ident(id),
                ) => {
                    let path = match path {
                        Some(mut path) => {
                            path.segments.push(syn::PathSegment::from(id));
                            path
                        }
                        None => {
                            let mut segments = Punctuated::new();
                            segments.push_value(id.into());

                            syn::Path {
                                leading_colon: Some(syn::Token![::]([
                                    colon1.span(),
                                    colon2.span(),
                                ])),
                                segments,
                            }
                        }
                    };

                    NsState::Path {
                        colon1: None,
                        colon2: None,
                        path: Some(path),
                    }
                }
                (
                    NsState::Path {
                        colon1: None,
                        colon2: None,
                        path: Some(path),
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == ',' => {
                    new.uri = Some(Namespace::Path(path));
                    NsState::Prefix
                }
                (
                    NsState::Path {
                        colon1: None,
                        colon2: None,
                        path: Some(path),
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == '=' => {
                    if path.leading_colon.is_some() {
                        panic!("prefix cannot be defined on a path in xml attribute");
                    }

                    if path.segments.len() != 1 {
                        panic!("prefix key must be a single identifier");
                    }

                    let segment = path.segments.into_iter().next().unwrap();
                    if !segment.arguments.is_empty() {
                        panic!("prefix key must be a single identifier without arguments");
                    }

                    NsState::PrefixValue {
                        prefix: segment.ident.to_string(),
                    }
                }
                (NsState::Prefix, TokenTree::Ident(id)) => NsState::Eq {
                    prefix: id.to_string(),
                },
                (NsState::Eq { mut prefix }, TokenTree::Punct(punct))
                    if punct.as_char() == '-' || punct.as_char() == '.' =>
                {
                    prefix.push(punct.as_char());
                    NsState::Eq { prefix }
                }
                (NsState::Eq { mut prefix }, TokenTree::Ident(id)) => {
                    prefix.push_str(&id.to_string());
                    NsState::Eq { prefix }
                }
                (NsState::Eq { prefix }, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                    NsState::PrefixValue { prefix }
                }
                (NsState::PrefixValue { prefix }, TokenTree::Literal(lit)) => {
                    new.prefixes
                        .insert(prefix.to_string(), Namespace::Literal(lit));
                    NsState::Comma
                }
                (NsState::PrefixValue { prefix }, TokenTree::Punct(punct))
                    if punct.as_char() == ':' =>
                {
                    NsState::PrefixPath {
                        prefix,
                        colon1: Some(punct),
                        colon2: None,
                        path: None,
                    }
                }
                (NsState::PrefixValue { prefix }, TokenTree::Ident(id)) => NsState::PrefixPath {
                    prefix,
                    colon1: None,
                    colon2: None,
                    path: Some(syn::Path::from(id)),
                },
                (
                    NsState::PrefixPath {
                        prefix,
                        colon1: None,
                        colon2: None,
                        path,
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == ':' => NsState::PrefixPath {
                    prefix,
                    colon1: Some(punct),
                    colon2: None,
                    path,
                },
                (
                    NsState::PrefixPath {
                        prefix,
                        colon1: colon1 @ Some(_),
                        colon2: None,
                        path,
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == ':' => NsState::PrefixPath {
                    prefix,
                    colon1,
                    colon2: Some(punct),
                    path,
                },
                (
                    NsState::PrefixPath {
                        prefix,
                        colon1: Some(colon1),
                        colon2: Some(colon2),
                        path,
                    },
                    TokenTree::Ident(id),
                ) => {
                    let path = match path {
                        Some(mut path) => {
                            path.segments.push(syn::PathSegment::from(id));
                            path
                        }
                        None => {
                            let mut segments = Punctuated::new();
                            segments.push_value(id.into());

                            syn::Path {
                                leading_colon: Some(syn::Token![::]([
                                    colon1.span(),
                                    colon2.span(),
                                ])),
                                segments,
                            }
                        }
                    };

                    NsState::PrefixPath {
                        prefix,
                        colon1: None,
                        colon2: None,
                        path: Some(path),
                    }
                }
                (
                    NsState::PrefixPath {
                        prefix,
                        colon1: None,
                        colon2: None,
                        path: Some(path),
                    },
                    TokenTree::Punct(punct),
                ) if punct.as_char() == ',' => {
                    new.prefixes
                        .insert(prefix.to_string(), Namespace::Path(path));
                    NsState::Prefix
                }
                (state, tree) => {
                    panic!(
                        "invalid state transition while parsing ns in xml attribute ({}, {tree})",
                        state.name()
                    )
                }
            };
        }

        match state {
            NsState::Start | NsState::Comma => {}
            NsState::Path {
                colon1: None,
                colon2: None,
                path: Some(path),
            } => {
                new.uri = Some(Namespace::Path(path));
            }
            NsState::PrefixPath {
                prefix,
                colon1: None,
                colon2: None,
                path: Some(path),
            } => {
                new.prefixes
                    .insert(prefix.to_string(), Namespace::Path(path));
            }
            state => panic!("invalid ns end state in xml attribute ({})", state.name()),
        }

        new
    }
}

pub enum NsState {
    Start,
    Comma,
    Path {
        colon1: Option<Punct>,
        colon2: Option<Punct>,
        path: Option<syn::Path>,
    },
    Prefix,
    Eq {
        prefix: String,
    },
    PrefixValue {
        prefix: String,
    },
    PrefixPath {
        prefix: String,
        colon1: Option<Punct>,
        colon2: Option<Punct>,
        path: Option<syn::Path>,
    },
}

impl NsState {
    pub fn name(&self) -> &'static str {
        match self {
            NsState::Start => "Start",
            NsState::Comma => "Comma",
            NsState::Path {
                colon1,
                colon2,
                path,
            } => match (colon1, colon2, path) {
                (None, None, None) => "Path [000]",
                (Some(_), None, None) => "Path [100]",
                (None, Some(_), None) => "Path [010]",
                (None, None, Some(_)) => "Path [001]",
                (Some(_), Some(_), None) => "Path [110]",
                (None, Some(_), Some(_)) => "Path [011]",
                (Some(_), None, Some(_)) => "Path [101]",
                (Some(_), Some(_), Some(_)) => "Path [111]",
            },
            NsState::Prefix => "Prefix",
            NsState::Eq { .. } => "Eq",
            NsState::PrefixValue { .. } => "PrefixValue",
            NsState::PrefixPath { .. } => "PrefixPath",
        }
    }
}
