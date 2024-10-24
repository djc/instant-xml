use std::collections::BTreeSet;

use proc_macro2::{Literal, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{DeriveInput, Generics};

use crate::case::RenameRule;

use super::{meta_items, MetaItem, NamespaceMeta};

pub struct ContainerMeta<'input> {
    pub input: &'input DeriveInput,
    pub ns: NamespaceMeta,
    pub rename: Option<Literal>,
    pub rename_all: RenameRule,
    pub mode: Option<Mode>,
}

impl<'input> ContainerMeta<'input> {
    pub fn from_derive(input: &'input syn::DeriveInput) -> Result<Self, syn::Error> {
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
                    Some(_) => return Err(syn::Error::new(span, "cannot have two modes")),
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

    pub fn xml_generics(&self, borrowed: BTreeSet<syn::Lifetime>) -> Generics {
        let mut xml_generics = self.input.generics.clone();
        let mut xml = syn::LifetimeParam::new(syn::Lifetime::new("'xml", Span::call_site()));
        xml.bounds.extend(borrowed);
        xml_generics.params.push(xml.into());

        for param in xml_generics.type_params_mut() {
            param
                .bounds
                .push(syn::parse_str("::instant_xml::FromXml<'xml>").unwrap());
        }

        xml_generics
    }

    pub fn tag(&self) -> TokenStream {
        match &self.rename {
            Some(name) => quote!(#name),
            None => self.input.ident.to_string().into_token_stream(),
        }
    }

    pub fn default_namespace(&self) -> TokenStream {
        match &self.ns.uri {
            Some(ns) => quote!(#ns),
            None => quote!(""),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Forward,
    Scalar,
    Transparent,
}
