extern crate proc_macro;

mod de;
mod se;

use std::collections::{BTreeSet, HashMap};

use proc_macro2::TokenStream;
use quote::quote;
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
        if let Some(NestedMeta::Lit(Lit::Str(v))) = iter.next() {
            default_namespace = v.value();
        }

        for item in iter {
            if let NestedMeta::Meta(Meta::NameValue(key)) = item {
                if let Lit::Str(value) = &key.lit {
                    other_namespaces
                        .insert(key.path.get_ident().unwrap().to_string(), value.value());
                    continue;
                }
            }
            panic!("Wrong data");
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
    let root_name = ident.to_string();
    let mut missing_prefixes = BTreeSet::new();
    let mut serializer = Serializer::new(&ast.attrs);
    let mut output = TokenStream::new();
    serializer.add_header(&mut output);

    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        serializer.process_named_field(field, &mut output, &mut missing_prefixes);
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    serializer.add_footer(&root_name, &mut output);

    let current_prefixes = serializer.keys_set();

    proc_macro::TokenStream::from(quote!(
        impl ToXml for #ident {
            fn serialize<W>(&self, serializer: &mut instant_xml::Serializer<W>, _field_data: Option<&instant_xml::FieldContext>) -> Result<(), instant_xml::Error>
            where
                W: std::fmt::Write,
            {
                let mut field_context = instant_xml::FieldContext {
                    name: #root_name,
                    attribute: None,
                };

                // Check if prefix exist
                #(
                    if serializer.parent_prefixes.get(#missing_prefixes).is_none() {
                        return Err(instant_xml::Error::WrongNamespace);
                    }
                )*;

                // Adding current prefixes
                let mut to_remove: Vec<&str> = Vec::new();
                #(if serializer.parent_prefixes.insert(#current_prefixes) {
                    to_remove.push(#current_prefixes);
                };)*;

                #output

                // Removing current prefixes
                for it in to_remove {
                    serializer.parent_prefixes.remove(it);
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
