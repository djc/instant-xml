extern crate proc_macro;

mod de;
mod ser;

use std::collections::BTreeMap;
use std::fmt;

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;
use syn::token::Colon2;

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    ser::to_xml(&ast).into()
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    proc_macro::TokenStream::from(de::from_xml(&ast))
}

#[derive(Default)]
struct ContainerMeta {
    ns: NamespaceMeta,
}

impl ContainerMeta {
    fn from_derive(input: &syn::DeriveInput) -> ContainerMeta {
        let mut meta = ContainerMeta::default();
        for item in meta_items(&input.attrs) {
            match item {
                MetaItem::Attribute => panic!("attribute key invalid in container xml attribute"),
                MetaItem::Ns(ns) => meta.ns = ns,
            }
        }
        meta
    }
}

#[derive(Default)]
struct FieldMeta {
    attribute: bool,
    ns: NamespaceMeta,
}

impl FieldMeta {
    fn from_field(input: &syn::Field) -> FieldMeta {
        let mut meta = FieldMeta::default();
        for item in meta_items(&input.attrs) {
            match item {
                MetaItem::Attribute => meta.attribute = true,
                MetaItem::Ns(ns) => meta.ns = ns,
            }
        }
        meta
    }
}

#[derive(Default)]
struct NamespaceMeta {
    uri: Option<Namespace>,
    prefixes: BTreeMap<String, Namespace>,
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

fn meta_items(attrs: &[syn::Attribute]) -> Vec<MetaItem> {
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
        state = match (state, tree) {
            (MetaState::Start, TokenTree::Ident(id)) => {
                if id == "attribute" {
                    items.push(MetaItem::Attribute);
                    MetaState::Comma
                } else if id == "ns" {
                    MetaState::Ns
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
                items.push(MetaItem::Ns(NamespaceMeta::from_tokens(group)));
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
}

impl MetaState {
    fn name(&self) -> &'static str {
        match self {
            MetaState::Start => "Start",
            MetaState::Comma => "Comma",
            MetaState::Ns => "Ns",
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

enum MetaItem {
    Ns(NamespaceMeta),
    Attribute,
}

enum Namespace {
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

fn discard_lifetimes(ty: &mut syn::Type) {
    match ty {
        syn::Type::Path(ty) => discard_path_lifetimes(ty),
        syn::Type::Reference(ty) => {
            ty.lifetime = None;
            discard_lifetimes(&mut ty.elem);
        }
        _ => {}
    }
}

fn discard_path_lifetimes(path: &mut syn::TypePath) {
    if let Some(q) = &mut path.qself {
        discard_lifetimes(&mut q.ty);
    }

    for segment in &mut path.path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(args) => {
                args.args.iter_mut().for_each(|arg| match arg {
                    syn::GenericArgument::Lifetime(lt) => {
                        *lt = syn::Lifetime::new("'_", Span::call_site())
                    }
                    syn::GenericArgument::Type(ty) => discard_lifetimes(ty),
                    syn::GenericArgument::Binding(_)
                    | syn::GenericArgument::Constraint(_)
                    | syn::GenericArgument::Const(_) => {}
                })
            }
            syn::PathArguments::Parenthesized(args) => {
                args.inputs.iter_mut().for_each(discard_lifetimes)
            }
        }
    }
}
