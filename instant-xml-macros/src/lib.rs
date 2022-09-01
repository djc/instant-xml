extern crate proc_macro;

mod de;
mod se;

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Lit, Meta, NestedMeta};

use crate::se::Serializer;

const XML: &str = "xml";

pub(crate) enum FieldAttribute {
    Namespace(String),
    PrefixIdentifier(String),
    Attribute,
}

pub(crate) fn get_namespaces(
    attributes: &Vec<syn::Attribute>,
) -> (String, HashMap<String, String>) {
    let mut default_namespace = String::new();
    let mut other_namespaces = HashMap::default();

    let (list, name) = match retrieve_attr_list(attributes) {
        Some((Some(list), name)) => (list, name),
        None => return (default_namespace, other_namespaces),
        _ => panic!("wrong parameters"),
    };

    if name == "namespace" {
        let mut iter = list.nested.iter();
        let mut next = iter.next();
        if let Some(NestedMeta::Lit(Lit::Str(v))) = next {
            default_namespace = v.value();
            next = iter.next();
        }

        while let Some(value) = next {
            if let NestedMeta::Meta(Meta::NameValue(key)) = value {
                if let Lit::Str(value) = &key.lit {
                    other_namespaces
                        .insert(key.path.get_ident().unwrap().to_string(), value.value());
                    next = iter.next();
                    continue;
                }
            }
            panic!("Wrong data")
        }
    }

    (default_namespace, other_namespaces)
}

pub(crate) fn retrieve_field_attribute(input: &syn::Field) -> Option<FieldAttribute> {
    match retrieve_attr_list(&input.attrs) {
        Some((Some(list), name)) if name.as_str() == "namespace" => match list.nested.first() {
            Some(NestedMeta::Lit(Lit::Str(v))) => Some(FieldAttribute::Namespace(v.value())),
            Some(NestedMeta::Meta(Meta::Path(v))) => {
                if let Some(ident) = v.get_ident() {
                    Some(FieldAttribute::PrefixIdentifier(ident.to_string()))
                } else {
                    panic!("unexpected parameter");
                }
            }
            _ => panic!("unexpected parameter"),
        },
        Some((None, name)) if name.as_str() == "attribute" => Some(FieldAttribute::Attribute),
        None => None,
        _ => panic!("unexpected parameter"),
    }
}

fn retrieve_attr_list(attributes: &Vec<syn::Attribute>) -> Option<(Option<syn::MetaList>, String)> {
    for attr in attributes {
        if !attr.path.is_ident(XML) {
            continue;
        }

        let nested = match attr.parse_meta() {
            Ok(Meta::List(meta)) => meta.nested,
            Ok(_) => todo!(),
            _ => todo!(),
        };

        let list = match nested.first() {
            Some(NestedMeta::Meta(Meta::List(list))) => list,
            Some(NestedMeta::Meta(Meta::Path(path))) => {
                return Some((None, path.get_ident()?.to_string()))
            }
            _ => return None,
        };

        return Some((Some(list.to_owned()), list.path.get_ident()?.to_string()));
    }

    None
}

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;
    let generics = (&ast.generics).into_token_stream();

    let root_name = ident.to_string();
    let mut serializer = Serializer::new(&ast.attrs);

    let mut header = TokenStream::new();
    serializer.add_header(&mut header);

    let mut body = TokenStream::new();
    let mut attributes = TokenStream::new();
    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        serializer.process_named_field(field, &mut body, &mut attributes);
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    let mut footer = TokenStream::new();
    serializer.add_footer(&root_name, &mut footer);

    let current_namespaces = serializer.namespaces_token();

    proc_macro::TokenStream::from(quote!(
        impl #generics ToXml for #ident #generics {
            fn serialize<W>(&self, serializer: &mut instant_xml::Serializer<W>) -> Result<(), instant_xml::Error>
            where
                W: std::fmt::Write,
            {
                let _ = serializer.consume_field_context();
                let mut field_context = instant_xml::FieldContext {
                    name: #root_name,
                    attribute: None,
                };

                #attributes

                #header
                #current_namespaces
                #body
                #footer

                // Removing current namespaces
                for it in to_remove {
                    serializer.parent_namespaces.remove(it);
                }

                Ok(())
            }
        };
    ))
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let deserializer = de::Deserializer::new(&ast);

    proc_macro::TokenStream::from(quote!(
        #deserializer
    ))
}
