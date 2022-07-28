extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{BTreeSet, HashMap};
use syn::{parse_macro_input, Lit, Meta, NestedMeta};

const XML: &str = "xml";

enum FieldAttribute {
    Namespace(String),
    PrefixIdentifier(String),
}

struct Serializer {
    default_namespace: Option<String>,
    other_namespaces: HashMap<String, String>,
}

impl<'a> Serializer {
    pub fn new(attributes: &'a Vec<syn::Attribute>) -> Serializer {
        let mut default_namespace = None;
        let mut other_namespaces = HashMap::default();

        if let Some(list) = Self::retrieve_namespace_list(attributes) {
            match list.path.get_ident() {
                Some(ident) if ident == "namespace" => {
                    let mut iter = list.nested.iter();
                    if let Some(NestedMeta::Lit(Lit::Str(v))) = iter.next() {
                        default_namespace = Some(v.value());
                    }

                    for item in iter {
                        match item {
                            NestedMeta::Meta(Meta::NameValue(key)) => {
                                if let Lit::Str(value) = &key.lit {
                                    other_namespaces.insert(
                                        key.path.get_ident().unwrap().to_string(),
                                        value.value(),
                                    );
                                }
                            }
                            _ => todo!(),
                        }
                    }
                }
                _ => (),
            }
        }

        Serializer {
            default_namespace,
            other_namespaces,
        }
    }

    fn keys_set(&self) -> BTreeSet<&str> {
        self.other_namespaces
            .iter()
            .map(|(k, _)| k.as_str())
            .collect()
    }

    fn add_header(&mut self, root_name: &str, output: &'a mut TokenStream) {
        output.extend(quote!(
            output.push_str("<");
            output.push_str(#root_name);
        ));

        if let Some(default_namespace) = self.default_namespace.as_ref() {
            output.extend(quote!(
                output.push_str(" xmlns=\"");
                output.push_str(#default_namespace);
                output.push_str("\"");
            ));
        }

        let mut sorted_values: Vec<_> = self.other_namespaces.iter().collect();
        sorted_values.sort();

        for (key, val) in sorted_values {
            output.extend(quote!(
                output.push_str(" xmlns:");
                output.push_str(#key);
                output.push_str("=\"");
                output.push_str(#val);
                output.push_str("\"");
            ));
        }
        output.extend(quote!(
            output.push_str(">");
        ));
    }

    fn add_footer(&mut self, root_name: &str, output: &'a mut TokenStream) {
        output.extend(quote!(
            output.push_str("</");
            output.push_str(#root_name);
            output.push_str(">");
        ));
    }

    fn process_named_field(
        &mut self,
        field: &syn::Field,
        output: &'a mut TokenStream,
        missing_prefixes: &'a mut BTreeSet<String>,
    ) {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();
        let mut prefix = String::default();

        match Self::retrieve_field_attribute(field) {
            Some(FieldAttribute::Namespace(namespace)) => {
                output.extend(quote!(
                    output.push_str("<");
                    output.push_str(#field_name);
                    output.push_str(" xmlns=\"");
                    output.push_str(#namespace);
                    output.push_str("\"");
                ));
            }
            Some(FieldAttribute::PrefixIdentifier(prefix_key)) => {
                output.extend(quote!(
                    output.push_str("<");
                    output.push_str(#prefix_key);
                    output.push_str(":");
                    output.push_str(#field_name);
                ));

                if self.other_namespaces.get(&prefix_key).is_none() {
                    missing_prefixes.insert(prefix_key.clone());
                };
                prefix = prefix_key + ":";
            }
            _ => {
                // Without the namespace
                output.extend(quote!(
                    output.push_str("<");
                    output.push_str(#field_name);
                ));
            }
        };

        output.extend(quote!(
            output.push_str(">");
            output.push_str(self.#field_value.to_xml(Some(child_prefixes)).unwrap().as_str());
            output.push_str("</");
            output.push_str(#prefix);
            output.push_str(#field_name);
            output.push_str(">");
        ));
    }

    fn retrieve_namespace_list(attributes: &Vec<syn::Attribute>) -> Option<syn::MetaList> {
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
                return Some(list.to_owned());
            }
        }

        None
    }

    fn retrieve_field_attribute(input: &syn::Field) -> Option<FieldAttribute> {
        if let Some(list) = Self::retrieve_namespace_list(&input.attrs) {
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
}

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;
    let root_name = ident.to_string();
    let mut missing_prefixes = BTreeSet::new();
    let mut serializer = Serializer::new(&ast.attrs);

    let mut output = TokenStream::new();
    output.extend(quote!(let mut output = String::new();));

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

    let current_prefixes: BTreeSet<&str> = serializer.keys_set();
    proc_macro::TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W, parent_prefixes: Option<&mut std::collections::BTreeSet<&str>>) -> Result<(), instant_xml::Error> {
                match parent_prefixes {
                    Some(child_prefixes) => {
                        let mut to_remove: Vec<&str> = Vec::new();
                        #(if child_prefixes.insert(#current_prefixes) {
                            to_remove.push(#current_prefixes);
                        };)*;
                        #output;
                        write.write_str(&(output))?;
                        for it in to_remove {
                            child_prefixes.remove(it);
                        }
                    },
                    None => {
                        let mut set = std::collections::BTreeSet::<&str>::new();
                        let child_prefixes = &mut set;
                        #(child_prefixes.insert(#current_prefixes);)*;
                        #output;
                        write.write_str(&(output))?;
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
