use crate::{retrieve_attr_list, retrieve_field_attribute, FieldAttribute};
use quote::quote;
use std::collections::{BTreeSet, HashMap};
use syn::{Lit, Meta, NestedMeta};

pub struct Serializer {
    default_namespace: Option<String>,
    other_namespaces: HashMap<String, String>,
}

impl<'a> Serializer {
    pub fn new(attributes: &'a Vec<syn::Attribute>) -> Serializer {
        let mut default_namespace = None;
        let mut other_namespaces = HashMap::default();

        if let Some(list) = retrieve_attr_list("namespace", attributes) {
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

    pub fn get_keys_set(&self) -> BTreeSet<&str> {
        self.other_namespaces
            .iter()
            .map(|(k, _)| k.as_str())
            .collect()
    }

    pub fn add_header(&mut self, root_name: &str, output: &'a mut proc_macro2::TokenStream) {
        output.extend(quote!(+ "<" + #root_name));

        if let Some(default_namespace) = self.default_namespace.as_ref() {
            output.extend(quote!(+ " xmlns=\"" + #default_namespace + "\""));
        }

        let mut sorted_values: Vec<_> = self.other_namespaces.iter().collect();
        sorted_values.sort();

        for (key, val) in sorted_values {
            output.extend(quote!(+ " xmlns:" + #key + "=\"" + #val + "\""));
        }

        output.extend(quote!(+ ">"));
    }

    pub fn add_footer(&mut self, root_name: &str, output: &'a mut proc_macro2::TokenStream) {
        output.extend(quote!(+ "</" + #root_name + ">"));
    }

    pub fn process_named_field(
        &mut self,
        field: &syn::Field,
        output: &'a mut proc_macro2::TokenStream,
        missing_prefixes: &'a mut BTreeSet<String>,
    ) {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();
        let mut prefix = String::default();

        match retrieve_field_attribute("namespace", field) {
            Some(FieldAttribute::Namespace(namespace)) => {
                output.extend(quote!(+ "<" + #field_name + " xmlns=\"" + #namespace + "\""));
            }
            Some(FieldAttribute::PrefixIdentifier(prefix_key)) => {
                output.extend(quote!(+ "<" + #prefix_key + ":" + #field_name));
                if self.other_namespaces.get(&prefix_key).is_none() {
                    missing_prefixes.insert(prefix_key.clone());
                };
                prefix = prefix_key + ":";
            }
            _ => {
                // Without the namespace
                output.extend(quote!(+ "<" + #field_name));
            }
        };

        output.extend(
            quote!(+ ">" + self.#field_value.to_xml(Some(child_prefixes)).unwrap().as_str() + "</" + #prefix + #field_name + ">"),
        );
    }
}
