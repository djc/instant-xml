use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::{get_namespaces, retrieve_field_attribute, FieldAttribute};

struct Tokens {
    enum_: TokenStream,
    consts: TokenStream,
    names: TokenStream,
    match_: TokenStream,
}

impl Default for Tokens {
    fn default() -> Self {
        Self {
            enum_: TokenStream::new(),
            consts: TokenStream::new(),
            names: TokenStream::new(),
            match_: TokenStream::new(),
        }
    }
}

pub struct Deserializer {
    out: TokenStream,
}

impl quote::ToTokens for Deserializer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.out.clone());
    }
}

impl Deserializer {
    pub fn new(input: &syn::DeriveInput) -> Deserializer {
        let ident = &input.ident;
        let name = ident.to_string();
        let mut out = TokenStream::new();

        let (default_namespace, other_namespaces) = get_namespaces(&input.attrs);
        let mut namespaces_map = quote!(let mut namespaces_map = std::collections::HashMap::new(););

        for (k, v) in other_namespaces.iter() {
            namespaces_map.extend(quote!(
                namespaces_map.insert(#k, #v);
            ))
        }

        // Varying values
        let mut elements_tokens = Tokens::default();
        let mut attributes_tokens = Tokens::default();

        // Common values
        let mut declare_values = TokenStream::new();
        let mut return_val = TokenStream::new();

        match &input.data {
            syn::Data::Struct(ref data) => {
                match data.fields {
                    syn::Fields::Named(ref fields) => {
                        fields.named.iter().enumerate().for_each(|(index, field)| {
                            let (tokens, def_prefix, is_element) = match retrieve_field_attribute(field) {
                                Some(FieldAttribute::Namespace(_)) => {
                                    todo!();
                                }
                                Some(FieldAttribute::PrefixIdentifier(def_prefix)) => {
                                    (&mut elements_tokens, Some(def_prefix), true)
                                },
                                Some(FieldAttribute::Attribute) => {
                                    (&mut attributes_tokens, None, false)
                                }
                                None => {
                                    (&mut elements_tokens, None, true)
                                },

                            };

                            Self::process_field(
                                field,
                                index,
                                &mut declare_values,
                                &mut return_val,
                                tokens,
                                is_element,
                                def_prefix,
                            );
                        });
                    }
                    syn::Fields::Unnamed(_) => todo!(),
                    syn::Fields::Unit => {}
                };
            }
            _ => todo!(),
        };

        // Elements
        let elements_enum = elements_tokens.enum_;
        let elements_consts = elements_tokens.consts;
        let elements_names = elements_tokens.names;
        let elem_type_match = elements_tokens.match_;

        // Attributes
        let attributes_enum = attributes_tokens.enum_;
        let attributes_consts = attributes_tokens.consts;
        let attributes_names = attributes_tokens.names;
        let attr_type_match = attributes_tokens.match_;

        out.extend(quote!(
            fn deserialize(deserializer: &mut ::instant_xml::Deserializer) -> Result<Self, ::instant_xml::Error> {
                println!("deserialize: {}", #name);
                use ::instant_xml::parse::XmlRecord;
                use ::instant_xml::{Error, Deserializer, Visitor} ;

                enum __Elements {
                    #elements_enum
                    __Ignore,
                }

                fn get_element(value: &str) -> __Elements {
                    #elements_consts
                    match value {
                        #elements_names
                        _ => __Elements::__Ignore
                    }
                }

                enum __Attributes {
                    #attributes_enum
                    __Ignore,
                }

                fn get_attribute(value: &str) -> __Attributes {
                    #attributes_consts
                    match value {
                        #attributes_names
                        _ => __Attributes::__Ignore
                    }
                }

                struct StructVisitor;
                impl<'xml> Visitor<'xml> for StructVisitor {
                    type Value = #ident;

                    fn visit_struct<'a>(&self, deserializer: &mut ::instant_xml::Deserializer) -> Result<Self::Value, ::instant_xml::Error>
                    {
                        #declare_values
                        println!("visit struct");

                        while let Some(( key, _ )) = deserializer.peek_next_attribute() {
                            match get_attribute(&key) {
                                #attr_type_match
                                __Attributes::__Ignore => todo!(),
                            }
                        }
                        while let Some(item) = &deserializer.peek_next_tag()? {
                            match item {
                                XmlRecord::Open(item) => {
                                    match get_element(&item.key.as_ref()) {
                                        #elem_type_match
                                        __Elements::__Ignore => todo!(),
                                    }
                                 }
                                 XmlRecord::Close(tag) => {
                                    println!("Close: {}", tag);
                                    if tag == &#name {
                                        break;
                                    }
                                },
                                XmlRecord::Element(_) => panic!("Unexpected element"),
                            }
                        }

                        println!("return");
                        Ok(Self::Value {
                            #return_val
                    })
                    }
                }

                #namespaces_map;
                deserializer.deserialize_struct(StructVisitor{}, #name, #default_namespace, &namespaces_map)
            }
        ));

        out.extend(quote!(
            const TAG_NAME: ::instant_xml::TagName<'xml> = ::instant_xml::TagName::Custom(#name);
        ));

        Deserializer { out }
    }

    fn process_field(
        field: &syn::Field,
        index: usize,
        declare_values: &mut TokenStream,
        return_val: &mut TokenStream,
        tokens: &mut Tokens,
        is_element: bool,
        def_prefix: Option<String>,
    ) {
        let field_var = field.ident.as_ref().unwrap();
        let field_var_str = field_var.to_string();
        let const_field_var_str = Ident::new(&field_var_str.to_uppercase(), Span::call_site());
        let field_type = match &field.ty {
            syn::Type::Path(v) => v.path.get_ident(),
            _ => todo!(),
        };

        let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
        tokens.enum_.extend(quote!(#enum_name,));

        tokens.consts.extend(quote!(
            const #const_field_var_str: &str = match #field_type::TAG_NAME {
                ::instant_xml::TagName::FieldName => #field_var_str,
                ::instant_xml::TagName::Custom(v) => v,
            };
        ));

        if is_element {
            tokens.names.extend(quote!(
                #const_field_var_str => __Elements::#enum_name,
            ));
        } else {
            tokens.names.extend(quote!(
                #const_field_var_str => __Attributes::#enum_name,
            ));
        }

        declare_values.extend(quote!(
            let mut #enum_name: Option<#field_type> = None;
        ));

        
        let def_prefix = match def_prefix {
            Some(def_prefix) => quote!(let def_prefix: Option<&str> = Some(#def_prefix);),
            None => quote!(let def_prefix: Option<&str> = None;),
        };

        if is_element {
            tokens.match_.extend(quote!(
                __Elements::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    match item.prefix {
                        Some(item) => {
                            let parser_prefix = item.to_owned();
                            #def_prefix
                            match def_prefix {
                                Some(def_prefix) => {
                                    if deserializer.get_parser_namespace(&parser_prefix)
                                        != deserializer.get_def_namespace(def_prefix) {
                                        return Err(Error::UnexpectedPrefix)
                                    }
                                } 
                                None => {
                                    return Err(Error::WrongNamespace)
                                }
                            }
                        }
                        None => {
                            if !deserializer.compare_parser_and_def_default_namespaces() {
                                return Err(Error::WrongNamespace)
                            }
                        }
                    }

                    #enum_name = Some(#field_type::deserialize(deserializer)?);
                },
            ));
        } else {
            tokens.match_.extend(quote!(
                __Attributes::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    deserializer.set_next_type_as_attribute()?;
                    #enum_name = Some(#field_type::deserialize(deserializer)?);
                },
            ));
        }

        return_val.extend(quote!(
            #field_var: #enum_name.expect("Expected some value"),
        ));
    }
}
