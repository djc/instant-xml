extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{parse_macro_input, DeriveInput, Lit, Meta, NestedMeta};

fn retrieve_namespace_list(
    attributes: &Vec<syn::Attribute>,
) -> Option<syn::punctuated::Punctuated<NestedMeta, syn::token::Comma>> {
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
            _ => todo!(),
        };

        if list.path.get_ident()? == "namespace" {
            return Some(list.nested.clone());
        }
    }

    None
}

fn retrieve_field_namespace(input: &syn::Field) -> Option<String> {
    if let Some(list) = retrieve_namespace_list(&input.attrs) {
        if let NestedMeta::Lit(Lit::Str(v)) = list.first()? {
            return Some(v.value());
        }
    }
    None
}

type DefaultNamespace = String;
type OtherNamespaces = HashMap<String, String>;
type Namespaces = Option<(Option<DefaultNamespace>, Option<OtherNamespaces>)>;

fn retrieve_namespaces(input: &DeriveInput) -> Namespaces {
    let mut default_namespace: Option<DefaultNamespace> = None;
    let mut other_namespaces = OtherNamespaces::default();

    if let Some(list) = retrieve_namespace_list(&input.attrs) {
        for item in list {
            match item {
                NestedMeta::Lit(Lit::Str(v)) => {
                    default_namespace = Some(v.value());
                }
                NestedMeta::Meta(Meta::NameValue(key)) => {
                    if let Lit::Str(value) = &key.lit {
                        other_namespaces
                            .insert(key.path.get_ident().unwrap().to_string(), value.value());
                    }
                }
                _ => (),
            }
        }
    }
    
    Some((
        default_namespace,
        if other_namespaces.is_empty() {
            None
        } else {
            Some(other_namespaces)
        },
    ))
}

const XML: &str = "xml";

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let ident = &ast.ident;
    let root_name = ident.to_string();

    let mut header: String = root_name.to_string();
    let (default_namespace, other_namespaces) = match retrieve_namespaces(&ast) {
        Some((Some(v1), Some(v2))) => (Some(v1), Some(v2)),
        Some((Some(v1), None)) => (Some(v1), None),
        Some((None, Some(v2))) => (None, Some(v2)),
        _ => (None, None),
    };

    if default_namespace.is_some() {
        header += format!(" xmlns=\"{}\"", default_namespace.as_ref().unwrap()).as_str();
    };

    let mut output: proc_macro2::TokenStream =
        TokenStream::from(quote!("<".to_owned() + #header + ">")).into();

    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields
                    .named
                    .iter()
                    .for_each(|field| {
                        let field_name = field.ident.as_ref().unwrap().to_string();
                        let field_value = field.ident.as_ref().unwrap();
                        if let Some(namespaces_map) = &other_namespaces {
                            if let Some(namespace_key) = retrieve_field_namespace(field) {
                                if let Some(namespace_value) = namespaces_map.get(&namespace_key) {
                                    output.extend(quote!(+ "<" + #field_name + " xmlns=" + #namespace_value));
                                }
                                else if let Some(default) = &default_namespace {
                                    // Not exist in the map, adding default one if exist
                                    output.extend(quote!(+ "<" + #field_name + " xmlns=" + #default));
                                } else {
                                    // Without the namespace
                                    output.extend(quote!(+ "<" + #field_name +));
                                }
                            }
                        } else {
                            // Without the namespace
                            output.extend(quote!(+ "<" + #field_name +));
                        }
                        output.extend(quote!(+ ">" + self.#field_value.to_string().as_str() + "</" + #field_name + ">"));
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    output.extend(quote!(+ "</" + #root_name + ">"));

    TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W) -> Result<(), instant_xml::Error> {
                write.write_str(&(#output))?;
                Ok(())
            }
        }
    ))
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::ItemStruct);
    let ident = &ast.ident;
    let name = ident.to_string();
    TokenStream::from(quote!(
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
