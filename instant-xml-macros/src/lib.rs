extern crate proc_macro;

mod case;
mod de;
mod ser;

use std::collections::BTreeMap;
use std::fmt;

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Colon2;
use syn::{parse_macro_input, DeriveInput, Generics};

use case::RenameRule;

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

struct ContainerMeta<'input> {
    input: &'input DeriveInput,
    ns: NamespaceMeta,
    rename: Option<Literal>,
    rename_all: RenameRule,
    mode: Option<Mode>,
}

impl<'input> ContainerMeta<'input> {
    fn from_derive(input: &'input syn::DeriveInput) -> Result<Self, syn::Error> {
        let mut ns = NamespaceMeta::default();
        let mut rename = Default::default();
        let mut rename_all = Default::default();
        let mut mode = None;

        for (item, span) in meta_items(&input.attrs) {
            match item {
                MetaItem::Ns(namespace) => ns = namespace,
                MetaItem::Rename(lit) => rename = Some(lit),
                MetaItem::RenameAll(lit) => {
                    rename_all = match RenameRule::from_str(&lit.to_string()) {
                        Ok(rule) => rule,
                        Err(err) => return Err(syn::Error::new(span, err)),
                    };
                }
                MetaItem::Mode(new) => match mode {
                    None => mode = Some(new),
                    Some(_) => return Err(syn::Error::new(span, "cannot have two enum modes")),
                },
                _ => {
                    return Err(syn::Error::new(
                        span,
                        "invalid field in container xml attribute",
                    ))
                }
            }
        }

        Ok(Self {
            input,
            ns,
            rename,
            rename_all,
            mode,
        })
    }

    fn xml_generics(&self) -> Generics {
        let mut xml_generics = self.input.generics.clone();
        let mut xml = syn::LifetimeDef::new(syn::Lifetime::new("'xml", Span::call_site()));
        xml.bounds
            .extend(xml_generics.lifetimes().map(|lt| lt.lifetime.clone()));
        xml_generics.params.push(xml.into());

        for param in xml_generics.type_params_mut() {
            param
                .bounds
                .push(syn::parse_str("::instant_xml::FromXml<'xml>").unwrap());
        }

        xml_generics
    }

    fn tag(&self) -> TokenStream {
        match &self.rename {
            Some(name) => quote!(#name),
            None => self.input.ident.to_string().into_token_stream(),
        }
    }

    fn default_namespace(&self) -> TokenStream {
        match &self.ns.uri {
            Some(ns) => quote!(#ns),
            None => quote!(""),
        }
    }
}

#[derive(Debug, Default)]
struct FieldMeta {
    attribute: bool,
    ns: NamespaceMeta,
    tag: TokenStream,
    serialize_with: Option<Literal>,
}

impl FieldMeta {
    fn from_field(input: &syn::Field, container: &ContainerMeta) -> Result<FieldMeta, syn::Error> {
        let field_name = input.ident.as_ref().unwrap();
        let mut meta = FieldMeta {
            tag: container
                .rename_all
                .apply_to_field(field_name)
                .into_token_stream(),
            ..Default::default()
        };

        for (item, span) in meta_items(&input.attrs) {
            match item {
                MetaItem::Attribute => meta.attribute = true,
                MetaItem::Ns(ns) => meta.ns = ns,
                MetaItem::Rename(lit) => meta.tag = quote!(#lit),
                MetaItem::SerializeWith(lit) => meta.serialize_with = Some(lit),
                MetaItem::RenameAll(_) => {
                    return Err(syn::Error::new(
                        span,
                        "attribute 'rename_all' invalid in field xml attribute",
                    ))
                }
                MetaItem::Mode(_) => {
                    return Err(syn::Error::new(span, "invalid attribute for struct field"));
                }
            }
        }

        Ok(meta)
    }
}

#[derive(Debug, Default)]
struct VariantMeta {
    serialize_as: TokenStream,
}

impl VariantMeta {
    fn from_variant(
        input: &syn::Variant,
        container: &ContainerMeta,
    ) -> Result<VariantMeta, syn::Error> {
        if !input.fields.is_empty() {
            return Err(syn::Error::new(
                input.fields.span(),
                "only unit enum variants are permitted!",
            ));
        }

        let mut rename = None;
        for (item, span) in meta_items(&input.attrs) {
            match item {
                MetaItem::Rename(lit) => rename = Some(lit.to_token_stream()),
                _ => {
                    return Err(syn::Error::new(
                        span,
                        "only 'rename' attribute is permitted on enum variants",
                    ))
                }
            }
        }

        let discriminant = match input.discriminant {
            Some((
                _,
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(ref lit),
                    ..
                }),
            )) => Some(lit.to_token_stream()),
            Some((
                _,
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(ref lit),
                    ..
                }),
            )) => Some(lit.base10_digits().to_token_stream()),
            Some((_, ref value)) => {
                return Err(syn::Error::new(
                    value.span(),
                    "invalid field discriminant value!",
                ))
            }
            None => None,
        };

        if discriminant.is_some() && rename.is_some() {
            return Err(syn::Error::new(
                input.span(),
                "conflicting `rename` attribute and variant discriminant!",
            ));
        }

        let serialize_as = match rename.or(discriminant) {
            Some(lit) => lit.into_token_stream(),
            None => container
                .rename_all
                .apply_to_variant(&input.ident)
                .to_token_stream(),
        };

        Ok(VariantMeta { serialize_as })
    }
}

#[derive(Debug, Default)]
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

fn meta_items(attrs: &[syn::Attribute]) -> Vec<(MetaItem, Span)> {
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

#[derive(Debug)]
enum MetaItem {
    Attribute,
    Ns(NamespaceMeta),
    Rename(Literal),
    Mode(Mode),
    RenameAll(Literal),
    SerializeWith(Literal),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Scalar,
    Wrapped,
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    #[test]
    fn non_unit_enum_variant_unsupported() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo(String),
                Bar,
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"only unit enum variants are permitted!\" }")
        .unwrap();
    }

    #[test]
    fn non_scalar_enums_unsupported() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml()]
            pub enum TestEnum {
                Foo,
                Bar,
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"missing enum mode\" }")
        .unwrap();
    }

    #[test]
    fn scalar_variant_attribute_not_permitted() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo,
                Bar,
                #[xml(scalar)]
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"only 'rename' attribute is permitted on enum variants\" }")
        .unwrap();
    }

    #[test]
    fn scalar_discrimintant_must_be_literal() {
        assert_eq!(
            None,
            dbg!(super::ser::to_xml(&parse_quote! {
                #[xml(scalar)]
                pub enum TestEnum {
                    Foo = 1,
                    Bar,
                    Baz
                }
            })
            .to_string())
            .find("compile_error ! { \"invalid field discriminant value!\" }")
        );

        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo = 1+1,
                Bar,
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"invalid field discriminant value!\" }")
        .unwrap();
    }

    #[test]
    fn rename_all_attribute_not_permitted() {
        dbg!(super::ser::to_xml(&parse_quote! {
            pub struct TestStruct {
                #[xml(rename_all = "UPPERCASE")]
                field_1: String,
                field_2: u8,
            }
        })
        .to_string())
        .find("compile_error ! { \"attribute 'rename_all' invalid in field xml attribute\" }")
        .unwrap();

        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo = 1,
                Bar,
                #[xml(rename_all = "UPPERCASE")]
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"only 'rename' attribute is permitted on enum variants\" }")
        .unwrap();
    }

    #[test]
    fn bogus_rename_all_not_permitted() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(rename_all = "forgetaboutit")]
            pub struct TestStruct {
                field_1: String,
                field_2: u8,
            }
        })
        .to_string())
        .find("compile_error ! {")
        .unwrap();
    }
}
