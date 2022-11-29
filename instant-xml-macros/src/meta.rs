use std::collections::BTreeMap;
use std::fmt;

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::token::Colon2;

use super::Mode;

#[derive(Debug, Default)]
pub(crate) struct NamespaceMeta {
    pub(crate) uri: Option<Namespace>,
    pub(crate) prefixes: BTreeMap<String, Namespace>,
}

impl NamespaceMeta {
    fn from_tokens(group: Group) -> Self {
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
                                leading_colon: Some(Colon2 {
                                    spans: [colon1.span(), colon2.span()],
                                }),
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
                        prefix: segment.ident,
                    }
                }
                (NsState::Prefix, TokenTree::Ident(id)) => NsState::Eq { prefix: id },
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
                                leading_colon: Some(Colon2 {
                                    spans: [colon1.span(), colon2.span()],
                                }),
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

pub(crate) fn meta_items(attrs: &[syn::Attribute]) -> Vec<(MetaItem, Span)> {
    let mut items = Vec::new();
    let attr = match attrs.iter().find(|attr| attr.path.is_ident("xml")) {
        Some(attr) => attr,
        None => return items,
    };

    let mut iter = attr.tokens.clone().into_iter();
    let first = match iter.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            group.stream()
        }
        _ => panic!("expected parenthesized group in xml attribute"),
    };

    if iter.next().is_some() {
        panic!("expected single token tree in xml attribute");
    }

    let mut state = MetaState::Start;
    for tree in first {
        let span = tree.span();
        state = match (state, tree) {
            (MetaState::Start, TokenTree::Ident(id)) => {
                if id == "attribute" {
                    items.push((MetaItem::Attribute, span));
                    MetaState::Comma
                } else if id == "borrow" {
                    items.push((MetaItem::Borrow, span));
                    MetaState::Comma
                } else if id == "ns" {
                    MetaState::Ns
                } else if id == "rename" {
                    MetaState::Rename
                } else if id == "rename_all" {
                    MetaState::RenameAll
                } else if id == "scalar" {
                    items.push((MetaItem::Mode(Mode::Scalar), span));
                    MetaState::Comma
                } else if id == "wrapped" {
                    items.push((MetaItem::Mode(Mode::Wrapped), span));
                    MetaState::Comma
                } else if id == "serialize_with" {
                    MetaState::SerializeWith
                } else if id == "deserialize_with" {
                    MetaState::DeserializeWith
                } else {
                    panic!("unexpected key in xml attribute");
                }
            }
            (MetaState::Comma, TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                MetaState::Start
            }
            (MetaState::Ns, TokenTree::Group(group))
                if group.delimiter() == Delimiter::Parenthesis =>
            {
                items.push((MetaItem::Ns(NamespaceMeta::from_tokens(group)), span));
                MetaState::Comma
            }
            (MetaState::Rename, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::RenameValue
            }
            (MetaState::RenameValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::Rename(lit), span));
                MetaState::Comma
            }
            (MetaState::RenameAll, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::RenameAllValue
            }
            (MetaState::RenameAllValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::RenameAll(lit), span));
                MetaState::Comma
            }
            (MetaState::SerializeWith, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::SerializeWithValue
            }
            (MetaState::SerializeWithValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::SerializeWith(lit), span));
                MetaState::Comma
            }
            (MetaState::DeserializeWith, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::DeserializeWithValue
            }
            (MetaState::DeserializeWithValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::DeserializeWith(lit), span));
                MetaState::Comma
            }
            (state, tree) => {
                panic!(
                    "invalid state transition while parsing xml attribute ({}, {tree})",
                    state.name()
                )
            }
        };
    }

    items
}

#[derive(Debug)]
enum MetaState {
    Start,
    Comma,
    Ns,
    Rename,
    RenameValue,
    RenameAll,
    RenameAllValue,
    SerializeWith,
    SerializeWithValue,
    DeserializeWith,
    DeserializeWithValue,
}

impl MetaState {
    fn name(&self) -> &'static str {
        match self {
            MetaState::Start => "Start",
            MetaState::Comma => "Comma",
            MetaState::Ns => "Ns",
            MetaState::Rename => "Rename",
            MetaState::RenameValue => "RenameValue",
            MetaState::RenameAll => "RenameAll",
            MetaState::RenameAllValue => "RenameAllValue",
            MetaState::SerializeWith => "SerializeWith",
            MetaState::SerializeWithValue => "SerializeWithValue",
            MetaState::DeserializeWith => "DeserializeWith",
            MetaState::DeserializeWithValue => "DeserializeWithValue",
        }
    }
}

enum NsState {
    Start,
    Comma,
    Path {
        colon1: Option<Punct>,
        colon2: Option<Punct>,
        path: Option<syn::Path>,
    },
    Prefix,
    Eq {
        prefix: Ident,
    },
    PrefixValue {
        prefix: Ident,
    },
    PrefixPath {
        prefix: Ident,
        colon1: Option<Punct>,
        colon2: Option<Punct>,
        path: Option<syn::Path>,
    },
}

impl NsState {
    fn name(&self) -> &'static str {
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

pub(crate) enum Namespace {
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

#[derive(Debug)]
pub(crate) enum MetaItem {
    Attribute,
    Borrow,
    Ns(NamespaceMeta),
    Rename(Literal),
    Mode(Mode),
    RenameAll(Literal),
    SerializeWith(Literal),
    DeserializeWith(Literal),
}
