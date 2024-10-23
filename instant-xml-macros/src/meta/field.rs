use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens};

use super::{meta_items, ContainerMeta, MetaItem, NamespaceMeta};

#[derive(Debug, Default)]
pub struct FieldMeta {
    pub attribute: bool,
    pub borrow: bool,
    pub direct: bool,
    pub ns: NamespaceMeta,
    pub tag: TokenStream,
    pub serialize_with: Option<Literal>,
    pub deserialize_with: Option<Literal>,
}

impl FieldMeta {
    pub fn from_field(
        input: &syn::Field,
        container: &ContainerMeta,
    ) -> Result<FieldMeta, syn::Error> {
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
                MetaItem::Borrow => meta.borrow = true,
                MetaItem::Direct => meta.direct = true,
                MetaItem::Ns(ns) => meta.ns = ns,
                MetaItem::Rename(lit) => meta.tag = quote!(#lit),
                MetaItem::SerializeWith(lit) => meta.serialize_with = Some(lit),
                MetaItem::DeserializeWith(lit) => meta.deserialize_with = Some(lit),
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
