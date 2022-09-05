extern crate proc_macro;

mod de;
mod ser;

use std::collections::HashMap;

use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Meta, NestedMeta};

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    ser::to_xml(&ast).into()
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let deserializer = de::Deserializer::new(&ast);

    proc_macro::TokenStream::from(quote!(
        #deserializer
    ))
}

#[derive(Debug, Default)]
struct ContainerMeta {
    ns: NamespaceMeta,
}

impl ContainerMeta {
    fn from_derive(input: &syn::DeriveInput) -> ContainerMeta {
        let mut meta = ContainerMeta::default();
        for item in meta_items(&input.attrs) {
            match item {
                Meta::List(list) if list.path.is_ident("ns") => {
                    meta.ns = NamespaceMeta::from_list(&list.nested)
                }
                _ => panic!("invalid xml attribute syntax"),
            }
        }
        meta
    }
}

#[derive(Debug, Default)]
struct FieldMeta {
    attribute: bool,
    ns: NamespaceMeta,
}

impl FieldMeta {
    fn from_field(input: &syn::Field) -> FieldMeta {
        let mut meta = FieldMeta::default();
        for item in meta_items(&input.attrs) {
            match item {
                Meta::Path(path) if path.is_ident("attribute") => meta.attribute = true,
                Meta::List(list) if list.path.is_ident("ns") => {
                    meta.ns = NamespaceMeta::from_list(&list.nested)
                }
                _ => panic!("invalid xml attribute syntax"),
            }
        }
        meta
    }
}

#[derive(Debug, Default)]
struct NamespaceMeta {
    default: Namespace,
    prefixes: HashMap<String, String>,
}

impl NamespaceMeta {
    fn from_list(list: &Punctuated<NestedMeta, syn::token::Comma>) -> NamespaceMeta {
        let mut meta = NamespaceMeta::default();
        for (i, item) in list.iter().enumerate() {
            match item {
                NestedMeta::Meta(inner) => match inner {
                    Meta::Path(path) => match path.get_ident() {
                        Some(id) => meta.default = Namespace::Prefix(id.to_string()),
                        None => panic!("invalid xml attribute syntax"),
                    },
                    Meta::NameValue(nv) => match (nv.path.get_ident(), &nv.lit) {
                        (Some(id), syn::Lit::Str(lit)) => {
                            meta.prefixes.insert(id.to_string(), lit.value());
                        }
                        _ => panic!("invalid xml attribute syntax"),
                    },
                    _ => panic!("invalid xml attribute syntax"),
                },
                NestedMeta::Lit(syn::Lit::Str(lit)) if i == 0 => {
                    meta.default = Namespace::Literal(lit.value())
                }
                _ => panic!("invalid xml attribute syntax"),
            }
        }
        meta
    }
}

fn meta_items(attrs: &[syn::Attribute]) -> impl Iterator<Item = Meta> + '_ {
    attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path.is_ident("xml") {
                return None;
            }

            match attr.parse_meta() {
                Ok(Meta::List(meta)) => Some(meta.nested.into_iter()),
                _ => panic!("unexpected xml attribute syntax"),
            }
        })
        .flatten()
        .map(|item| match item {
            NestedMeta::Meta(item) => item,
            NestedMeta::Lit(_) => panic!("unexpected xml attribute syntax"),
        })
}

#[derive(Debug)]
enum Namespace {
    Default,
    Prefix(String),
    Literal(String),
}

impl Default for Namespace {
    fn default() -> Self {
        Namespace::Default
    }
}
