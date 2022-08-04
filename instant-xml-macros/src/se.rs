use crate::{get_namespaces, retrieve_field_attribute, FieldAttribute};
use quote::quote;
use std::collections::{BTreeSet, HashMap};
use proc_macro2::TokenStream;

pub struct Serializer {
    default_namespace: Option<String>,
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

    pub fn keys_set(&self) -> BTreeSet<&str> {
        self.other_namespaces
            .iter()
            .map(|(k, _)| k.as_str())
            .collect()
    }

    pub fn add_header(&mut self, root_name: &str, output: &'a mut TokenStream) {
        output.extend(quote!(
            serializer.output.write_char('<')?;
            serializer.output.write_str(#root_name)?;
        ));

        if let Some(default_namespace) = self.default_namespace.as_ref() {
            output.extend(quote!(
                serializer.output.write_str(" xmlns=\"")?;
                serializer.output.write_str(#default_namespace)?;
                serializer.output.write_char('\"')?;
            ));
        }

        let mut sorted_values: Vec<_> = self.other_namespaces.iter().collect();
        sorted_values.sort();

        for (key, val) in sorted_values {
            output.extend(quote!(
                serializer.output.write_str(" xmlns:")?;
                serializer.output.write_str(#key)?;
                serializer.output.write_str("=\"")?;
                serializer.output.write_str(#val)?;
                serializer.output.write_char('\"')?;
            ));
        }
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

    pub fn process_named_field(
        &mut self,
        field: &syn::Field,
        output: &'a mut TokenStream,
        missing_prefixes: &'a mut BTreeSet<String>,
    ) {
        let name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();

        output.extend(quote!(
            let mut field = instant_xml::FieldContext {
                name: #name,
                attribute: None,
            };
        ));

        match retrieve_field_attribute("namespace", field) {
            Some(FieldAttribute::Namespace(namespace_key)) => {
                output.extend(quote!(
                    field.attribute = Some(instant_xml::FieldAttribute::Namespace(#namespace_key));
                ));
            }
            Some(FieldAttribute::PrefixIdentifier(prefix_key)) => {
                output.extend(quote!(
                    field.attribute = Some(instant_xml::FieldAttribute::Prefix(#prefix_key));
                ));

                if self.other_namespaces.get(&prefix_key).is_none() {
                    missing_prefixes.insert(prefix_key);
                };
            }
            _ => {}
        };

        output.extend(quote!(
            self.#field_value.serialize(serializer, Some(&field))?;
        ));
    }
}
