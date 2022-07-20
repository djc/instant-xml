extern crate proc_macro;

mod de;
mod se;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::parse_macro_input;
use syn::{Lit, Meta, NestedMeta};

const XML: &str = "xml";

pub(crate) enum FieldAttribute {
    Namespace(String),
    PrefixIdentifier(String),
}

pub(crate) fn retrieve_attr_list(
    name: &str,
    attributes: &Vec<syn::Attribute>,
) -> Option<syn::MetaList> {
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

        if list.path.get_ident()? == name {
            return Some(list.to_owned());
        }
    }

    None
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

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;
    let root_name = ident.to_string();
    let mut output: proc_macro2::TokenStream = TokenStream::from(quote!("".to_owned())).into();
    let mut missing_prefixes = BTreeSet::new();

    let mut serializer = se::Serializer::new(&ast.attrs);
    serializer.add_header(&root_name, &mut output);

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

    let current_prefixes: BTreeSet<&str> = serializer.get_keys_set();
    TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W, parent_prefixes: Option<&mut std::collections::BTreeSet<&str>>) -> Result<(), instant_xml::Error> {
                match parent_prefixes {
                    Some(child_prefixes) => {
                        let mut to_remove: Vec<&str> = Vec::new();
                        #(if child_prefixes.insert(#current_prefixes) {
                            to_remove.push(#current_prefixes);
                        };)*;
                        write.write_str(&(#output))?;

                        for it in to_remove {
                            child_prefixes.remove(it);
                        }
                    },
                    None => {
                        let mut set = std::collections::BTreeSet::<&str>::new();
                        let child_prefixes = &mut set;
                        #(child_prefixes.insert(#current_prefixes);)*;
                        write.write_str(&(#output))?;
                    }
                }
                Ok(())
            }

            fn to_xml(&self, parent_prefixes: Option<&mut std::collections::BTreeSet<&str>>) -> Result<String, instant_xml::Error> {
                //#(println!("{}", #missing_prefixes);)*;
                if let Some(parent_prefixes) = parent_prefixes.as_ref() {
                    #(
                        if parent_prefixes.get(#missing_prefixes).is_none() {
                            panic!("wrong prefix");
                        }
                    )*;
                }

                let mut out = String::new();
                self.write_xml(&mut out, parent_prefixes)?;
                Ok(out)
            }
        };
    ))
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;

    let deserializer = de::Deserializer::new(&ast);
    let fn_deserialize = deserializer.fn_deserialize;

    TokenStream::from(quote!(
        impl<'xml> FromXml<'xml> for #ident {
            #fn_deserialize
        }
    ))
}
