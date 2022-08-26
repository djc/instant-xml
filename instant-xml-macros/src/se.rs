use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;

use crate::{get_namespaces, retrieve_field_attribute, FieldAttribute};

pub struct Serializer {
    default_namespace: String,
    other_namespaces: HashMap<String, String>,
}

impl<'a> Serializer {
    pub fn new(attributes: &'a Vec<syn::Attribute>) -> Serializer {
        let (default_namespace, other_namespaces) = get_namespaces(attributes);

        Serializer {
            default_namespace,
            other_namespaces,
        }
    }

    pub fn add_header(&mut self, output: &'a mut TokenStream) {
        output.extend(quote!(
            serializer.output.write_char('<')?;
            serializer.output.write_str(field_context.name)?;
        ));

        let default_namespace = &self.default_namespace;
        output.extend(quote!(
            if serializer.parent_default_namespace != Some(#default_namespace) {
                serializer.output.write_str(" xmlns=\"")?;
                serializer.output.write_str(#default_namespace)?;
                serializer.output.write_char('\"')?;
            }
            serializer.parent_default_namespace = Some(#default_namespace);
        ));

        let mut sorted_values: Vec<_> = self.other_namespaces.iter().collect();
        sorted_values.sort();

        for (key, val) in sorted_values {
            output.extend(quote!(
               if serializer.parent_namespaces.get(#val).is_none() {
                        serializer.output.write_str(" xmlns:")?;
                        serializer.output.write_str(#key)?;
                        serializer.output.write_str("=\"")?;
                        serializer.output.write_str(#val)?;
                        serializer.output.write_char('\"')?;
                }
            ));
        }

        // Attributes
        output.extend(quote!(
            serializer.consume_current_attributes()?;
        ));

        output.extend(quote!(
            serializer.output.write_char('>')?;
        ));
    }

    pub fn add_footer(&mut self, root_name: &str, output: &'a mut TokenStream) {
        output.extend(quote!(
            serializer.output.write_str("</")?;
            serializer.output.write_str(#root_name)?;
            serializer.output.write_char('>')?;
        ));
    }

    pub fn process_named_field(&mut self, field: &syn::Field) -> (TokenStream, bool) {
        let name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();
        let mut is_attribute = false;
        let mut output = quote!(
            let mut field = instant_xml::FieldContext {
                name: #name,
                attribute: None,
            };
        );

        match retrieve_field_attribute(field) {
            Some(FieldAttribute::Namespace(namespace)) => {
                output.extend(quote!(
                        match serializer.parent_namespaces.get(#namespace) {
                            Some(key) => field.attribute = Some(instant_xml::FieldAttribute::Prefix(key)),
                            None => field.attribute = Some(instant_xml::FieldAttribute::Namespace(#namespace)),
                        };
                    )
                );
            }
            Some(FieldAttribute::PrefixIdentifier(prefix_key)) => {
                match self.other_namespaces.get(&prefix_key) {
                    Some(val) => {
                        output.extend(quote!(
                            let prefix_key = match serializer.parent_namespaces.get(#val) {
                                Some(key) => key,
                                None => #prefix_key,
                            };
                        ));
                    }
                    None => panic!("Prefix not defined: {}", prefix_key),
                };

                output.extend(quote!(
                    field.attribute = Some(instant_xml::FieldAttribute::Prefix(prefix_key));
                ));
            }
            Some(FieldAttribute::Attribute) => {
                is_attribute = true;
                output.extend(quote!(
                    serializer.add_attribute_key(&#name);
                    field.attribute = Some(instant_xml::FieldAttribute::Attribute);
                ));
            }
            _ => {}
        };

        output.extend(quote!(
            serializer.set_field_context(field)?;
            self.#field_value.serialize(serializer)?;
        ));

        (output, is_attribute)
    }

    pub fn get_namespaces_token(&self) -> TokenStream {
        let mut namespaces = quote!(
            let mut to_remove: Vec<&str> = Vec::new();
        );
        for (k, v) in self.other_namespaces.iter() {
            namespaces.extend(quote!(
                if let std::collections::hash_map::Entry::Vacant(v) = serializer.parent_namespaces.entry(#v) {
                        v.insert(#k);
                        to_remove.push(#v);
                };
            ))
        }
        namespaces
    }
}
