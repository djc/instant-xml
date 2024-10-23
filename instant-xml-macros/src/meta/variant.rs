use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;

use super::{meta_items, ContainerMeta, MetaItem};

#[derive(Debug, Default)]
pub struct VariantMeta {
    pub serialize_as: TokenStream,
}

impl VariantMeta {
    pub fn from_variant(
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
