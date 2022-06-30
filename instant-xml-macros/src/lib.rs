extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
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

    fn get_vec_keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        for keys in self.other_namespaces.keys() {
            out.push(keys.clone());
        }
        out
    }

    fn add_header(&mut self, root_name: &str, output: &'a mut proc_macro2::TokenStream) {
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

    fn add_footer(&mut self, root_name: &str, output: &'a mut proc_macro2::TokenStream) {
        output.extend(quote!(+ "</" + #root_name + ">"));
    }

    fn process_named_field(
        &mut self,
        field: &syn::Field,
        output: &'a mut proc_macro2::TokenStream,
        missing_prefixes: &'a mut Vec<String>,
    ) {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();
        let mut prefix = String::default();

        match Self::retrieve_field_attribute(field) {
            Some(FieldAttribute::Namespace(namespace)) => {
                output.extend(quote!(+ "<" + #field_name + " xmlns=\"" + #namespace + "\""));
            }
            Some(FieldAttribute::PrefixIdentifier(prefix_key)) => {
                output.extend(quote!(+ "<" + #prefix_key + ":" + #field_name));
                if self.other_namespaces.get(&prefix_key).is_none() {
                    missing_prefixes.push(prefix_key.clone());
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
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;
    let root_name = ident.to_string();
    let mut output: proc_macro2::TokenStream = TokenStream::from(quote!("".to_owned())).into();
    let mut missing_prefixes = Vec::new();

    let mut serializer = Serializer::new(&ast.attrs);
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

    let current_prefixes: Vec<String> = serializer.get_vec_keys();

    TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W, parent_prefixes: Option<Vec<String>>) -> Result<(), instant_xml::Error> {
                // #(println!("{}", #parent_prefixes);)*;
                let mut child_prefixes: Vec<String> = Vec::new();
                if let Some(parent_prefixes) = parent_prefixes {
                    child_prefixes = parent_prefixes;
                }

                #(child_prefixes.push(#current_prefixes.to_string());)*;

                write.write_str(&(#output))?;
                Ok(())
            }

            fn to_xml(&self, parent_prefixes: Option<Vec<String>>) -> Result<String, instant_xml::Error> {
                //#(println!("{}", #missing_prefixes);)*;
                if let Some(parent_prefixes) = parent_prefixes.as_ref() {
                    #(
                        if parent_prefixes.iter().find(|&value| value == #missing_prefixes).is_none() {
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
