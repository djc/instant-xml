extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
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

    fn get_keys_set(&self) -> BTreeSet<&str> {
        self.other_namespaces
            .iter()
            .map(|(k, _)| k.as_str())
            .collect()
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
        missing_prefixes: &'a mut BTreeSet<String>,
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
    let mut missing_prefixes = BTreeSet::new();

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
    let name = ident.to_string();
    let mut return_val: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
    let mut hash_map: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
    let mut enum_types: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
    let mut type_match: proc_macro2::TokenStream = TokenStream::from(quote!()).into();

    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        let mut field_name = field.ident.as_ref().unwrap().to_string();
                        let field_value = field.ident.as_ref().unwrap();
                        let field_type = if let syn::Type::Path(v) = &field.ty {
                            v.path.get_ident()
                        } else {
                            todo!();
                        };

                        // TODO: Change it to check against all scalar types
                        if *field_type.as_ref().unwrap() != "bool" {
                            field_name = field_type.as_ref().unwrap().to_string();
                        }

                        let enum_type = Ident::new(
                            &format!("T{}", field_type.as_ref().unwrap()),
                            Span::call_site(),
                        );
                        enum_types.extend(quote!(#enum_type(Option<#field_type>),));

                        hash_map.extend(quote!(
                            map.insert(#field_name.to_string(), Types::#enum_type(None));
                        ));

                        type_match.extend(quote!(
                            Types::#enum_type(None) => {
                                let value = Types::#enum_type(Some(#field_type::from_xml(&input, Some(&mut iter_ref), Some(value_from_parser)).unwrap()));
                                map.insert(#field_name.to_string(), value);
                            },
                        ));

                        return_val.extend(quote!(
                            #field_value: match map.remove(#field_name).expect("some value") {
                                Types::#enum_type(Some(v)) => v,
                                Types::#enum_type(None) => panic!("missing value {}", #field_name),
                                _ => panic!("wrong type"),
                            },
                        ));
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    TokenStream::from(quote!(
        impl<'xml> FromXml<'xml> for #ident {
            fn from_xml<'a>(input: &'a str, parent_iter: Option<&mut std::iter::Peekable<::instant_xml::xmlparser::Tokenizer<'a>>>, _scalar_type_value: Option<String>) -> Result<Self, ::instant_xml::Error> {
                use ::instant_xml::parse::{Parse, ParsingState, StackEntity, EndType};

                enum Types {
                    #enum_types
                }

                let mut map = std::collections::HashMap::<String, Types>::new();;
                #hash_map

                let mut iter = ::instant_xml::xmlparser::Tokenizer::from(input).peekable();
                let mut iter_ref = &mut iter;

                let mut stack = Vec::new();
                if parent_iter.is_some() {
                    iter_ref = parent_iter.unwrap();
                    stack.push(#name.to_string());
                }

                let mut key = None;
                let mut state = ParsingState::default();
                let mut value_from_parser = "".to_string();

                while let Some(item) = iter_ref.next() {
                    println!("{:?}", &item);
                    match item.get_next_element(&mut state) {
                        Some((StackEntity::Open, tag)) => {
                            key = Some(tag.to_string());
                            continue;
                        },
                        Some((StackEntity::Close(close_type), tag)) => {
                            match close_type {
                                EndType::Open => {
                                    stack.push(tag);
                                    state = ParsingState::default();
                                    println!("Stack size after push: {}", stack.len());

                                    let mut temp = ParsingState::default();
                                    match iter_ref.peek().unwrap().get_next_element(&mut temp) {
                                        Some((StackEntity::Element, _)) => continue,
                                        Some((StackEntity::Open, _)) => {
                                            if stack.len() <= 1 {
                                                continue;
                                            }
                                        },
                                        _ => todo!(),
                                    }
                                },
                                EndType::Close => {
                                    // TODO: Check if close tag equal to tag in top of stack
                                    stack.pop();

                                    println!("Stack size after pop: {}", stack.len());

                                    if stack.is_empty() {
                                        break;
                                    }

                                },
                                EndType::Empty => {
                                    continue;
                                },
                            }
                        },
                        Some((StackEntity::Attribute, s)) => {
                            // TODO: Add to attributes map
                            continue;
                        },
                        Some((StackEntity::Element, value)) => {
                            value_from_parser = value.to_string();
                            continue;
                        },
                        _ => {
                            continue;
                        },
                    }


                let key_temp: String = key.as_ref().expect("Valid key").clone();

                println!("To match: {}, {}", &value_from_parser, &key_temp);

                // This means that this is non-scalar field
                if value_from_parser.is_empty() {
                    stack.pop();
                }

                let type_from_map = map.get(key_temp.as_str()).unwrap();
                match type_from_map {
                    #type_match
                    _ => todo!(),
                };

                key = None;
                value_from_parser = "".to_string();
            }

                println!("return");
                Ok(Self {
                    #return_val
                })
            }
        }
    ))
}
