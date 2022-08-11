extern crate proc_macro;

mod se;

use std::collections::{BTreeSet, HashMap};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Lit, Meta, NestedMeta};

use crate::se::Serializer;

const XML: &str = "xml";

enum FieldAttribute {
    Namespace(String),
    PrefixIdentifier(String),
}

pub(crate) fn get_namespaces(
    attributes: &Vec<syn::Attribute>,
) -> (Option<String>, HashMap<String, String>) {
    let mut default_namespace = None;
    let mut other_namespaces = HashMap::default();

    let list = match retrieve_attr_list("namespace", attributes) {
        Some(v) => v,
        None => return (default_namespace, other_namespaces),
    };

    if list.path.get_ident().unwrap() == "namespace" {
        let mut iter = list.nested.iter();
        if let Some(NestedMeta::Lit(Lit::Str(v))) = iter.next() {
            default_namespace = Some(v.value());
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

pub(crate) fn retrieve_field_attribute(name: &str, input: &syn::Field) -> Option<FieldAttribute> {
    if let Some(list) = retrieve_attr_list(name, &input.attrs) {
        match list.nested.first() {
            Some(NestedMeta::Lit(Lit::Str(v))) => {
                return Some(FieldAttribute::Namespace(v.value()));
            }
            Some(NestedMeta::Meta(Meta::Path(v))) => {
                if let Some(ident) = v.get_ident() {
                    return Some(FieldAttribute::PrefixIdentifier(ident.to_string()));
                }
            }
            _ => (),
        };
    }
    None
}

fn retrieve_attr_list(name: &str, attributes: &Vec<syn::Attribute>) -> Option<syn::MetaList> {
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
            _ => return None,
        };

        if list.path.get_ident()? == name {
            return Some(list.to_owned());
        }
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
                        return Err(instant_xml::Error::WrongPrefix);
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
    let ast = parse_macro_input!(input as syn::ItemStruct);
    let ident = &ast.ident;
    let name = ident.to_string();
    proc_macro::TokenStream::from(quote!(
        impl<'xml> FromXml<'xml> for #ident {
            fn from_xml(input: &str) -> Result<Self, ::instant_xml::Error> {
                use ::instant_xml::parse::Parse;
                let mut iter = ::instant_xml::xmlparser::Tokenizer::from(input);
                iter.next().element_start(None, #name)?;
                iter.next().element_end(None, #name)?;
                Ok(Self)
            }
        }
    ))
}
