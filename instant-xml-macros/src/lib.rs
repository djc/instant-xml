extern crate proc_macro;

use std::collections::BTreeSet;
use std::mem;

use proc_macro2::{Literal, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Generics};

mod case;
use case::RenameRule;
mod de;
mod meta;
use meta::{meta_items, MetaItem, Namespace, NamespaceMeta};
mod ser;

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

    fn xml_generics<'a>(&self, borrowed: BTreeSet<syn::Lifetime>) -> Generics {
        let mut xml_generics = self.input.generics.clone();
        let mut xml = syn::LifetimeDef::new(syn::Lifetime::new("'xml", Span::call_site()));
        xml.bounds.extend(borrowed.into_iter());
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
    borrow: bool,
    direct: bool,
    ns: NamespaceMeta,
    tag: TokenStream,
    serialize_with: Option<Literal>,
    deserialize_with: Option<Literal>,
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

fn discard_lifetimes(
    ty: &mut syn::Type,
    borrowed: &mut BTreeSet<syn::Lifetime>,
    borrow: bool,
    top: bool,
) {
    match ty {
        syn::Type::Path(ty) => discard_path_lifetimes(ty, borrowed, borrow),
        syn::Type::Reference(ty) => {
            if top {
                // If at the top level, we'll want to borrow from `&'a str` and `&'a [u8]`.
                match &*ty.elem {
                    syn::Type::Path(inner) if top && inner.path.is_ident("str") => {
                        if let Some(lt) = ty.lifetime.take() {
                            borrowed.insert(lt);
                        }
                    }
                    syn::Type::Slice(inner) if top => match &*inner.elem {
                        syn::Type::Path(inner) if inner.path.is_ident("u8") => {
                            borrowed.extend(ty.lifetime.take().into_iter());
                        }
                        _ => {}
                    },
                    _ => {}
                }
            } else if borrow {
                // Otherwise, only borrow if the user has requested it.
                borrowed.extend(ty.lifetime.take().into_iter());
            } else {
                ty.lifetime = None;
            }

            discard_lifetimes(&mut ty.elem, borrowed, borrow, false);
        }
        _ => {}
    }
}

fn discard_path_lifetimes(
    path: &mut syn::TypePath,
    borrowed: &mut BTreeSet<syn::Lifetime>,
    borrow: bool,
) {
    if let Some(q) = &mut path.qself {
        discard_lifetimes(&mut q.ty, borrowed, borrow, false);
    }

    for segment in &mut path.path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(args) => {
                args.args.iter_mut().for_each(|arg| match arg {
                    syn::GenericArgument::Lifetime(lt) => {
                        let lt = mem::replace(lt, syn::Lifetime::new("'_", Span::call_site()));
                        if borrow {
                            borrowed.insert(lt);
                        }
                    }
                    syn::GenericArgument::Type(ty) => {
                        discard_lifetimes(ty, borrowed, borrow, false)
                    }
                    syn::GenericArgument::Binding(_)
                    | syn::GenericArgument::Constraint(_)
                    | syn::GenericArgument::Const(_) => {}
                })
            }
            syn::PathArguments::Parenthesized(args) => args
                .inputs
                .iter_mut()
                .for_each(|ty| discard_lifetimes(ty, borrowed, borrow, false)),
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
