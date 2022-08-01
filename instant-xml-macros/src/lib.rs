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
            serializer.output.push('<');
            serializer.output.push_str(#root_name);
        ));

        if let Some(default_namespace) = self.default_namespace.as_ref() {
            output.extend(quote!(
                serializer.output.push_str(" xmlns=\"");
                serializer.output.push_str(#default_namespace);
                serializer.output.push('\"');
            ));
        }

        let mut sorted_values: Vec<_> = self.other_namespaces.iter().collect();
        sorted_values.sort();

        for (key, val) in sorted_values {
            output.extend(quote!(
                serializer.output.push_str(" xmlns:");
                serializer.output.push_str(#key);
                serializer.output.push_str("=\"");
                serializer.output.push_str(#val);
                serializer.output.push('\"');
            ));
        }
        output.extend(quote!(
            serializer.output.push('>');
        ));
    }

    fn add_footer(&mut self, root_name: &str, output: &'a mut TokenStream) {
        output.extend(quote!(
            serializer.output.push_str("</");
            serializer.output.push_str(#root_name);
            serializer.output.push('>');
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

        output.extend(quote!(
            let mut field = instant_xml::FieldData {
                field_name: #field_name,
                field_attribute: None,
                value: "".to_owned(),
            };
        ));

        match Self::retrieve_field_attribute(field) {
            Some(FieldAttribute::Namespace(namespace_key)) => {
                output.extend(quote!(
                    field.field_attribute = Some(instant_xml::FieldAttribute::Namespace(#namespace_key));
                ));
            }
            Some(FieldAttribute::PrefixIdentifier(prefix_key)) => {
                output.extend(quote!(
                    field.field_attribute = Some(instant_xml::FieldAttribute::Prefix(#prefix_key));
                ));

                if self.other_namespaces.get(&prefix_key).is_none() {
                    missing_prefixes.insert(prefix_key);
                };
            }
            _ => {}
        };

        output.extend(quote!(
            self.#field_value.serialize(serializer, Some(&mut field))?;
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

    let current_prefixes = serializer.keys_set();
    proc_macro::TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml(&self, serializer: &mut instant_xml::Serializer, field_data: &instant_xml::FieldData) -> Result<(), instant_xml::Error> {
                // Check if prefix exist
                #(
                    if serializer.parent_prefixes.get(#missing_prefixes).is_none() {
                        panic!("wrong prefix");
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

            fn serialize(&self, serializer: &mut instant_xml::Serializer, _field_data: Option<&mut instant_xml::FieldData>) -> Result<(), instant_xml::Error> {
                let mut field_data = instant_xml::FieldData {
                    field_name: #root_name,
                    field_attribute: None,
                    value: "".to_owned(),
                };

                self.write_xml(serializer, &field_data)?;
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
