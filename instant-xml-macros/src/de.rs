use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::{get_namespaces, retrieve_attr};

struct Tokens {
    enum_: TokenStream,
    consts: TokenStream,
    names: TokenStream,
    match_: TokenStream,
}

impl Tokens {
    fn extend(&mut self, tokens: Tokens) {
        self.enum_.extend(tokens.enum_);
        self.consts.extend(tokens.consts);
        self.names.extend(tokens.names);
        self.match_.extend(tokens.match_);
    } 
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
        let vec = &self.out;
        tokens.extend(quote!(
            #vec
        ));
    }
}

enum FieldType {
    Attribute(Tokens),
    Element(Tokens),
}

impl Deserializer {
    pub fn new(input: &syn::DeriveInput) -> Deserializer {
        let ident = &input.ident;
        let name = ident.to_string();
        let mut out = TokenStream::new();

        let (_, other_namespaces) = get_namespaces(&input.attrs);
        let mut namespaces_map: TokenStream = proc_macro::TokenStream::from(
            quote!(let mut namespaces_map = std::collections::HashMap::new();),
        )
        .into();
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
                            match Self::process_field(
                                field,
                                index,
                                &mut declare_values,
                                &mut return_val,
                            ) {
                                FieldType::Element(tokens) => {
                                    elements_tokens.extend(tokens);
                                },
                                FieldType::Attribute(tokens) => {
                                    attributes_tokens.extend(tokens);
                                },
                            }
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
            fn deserialize(deserializer: &mut ::instant_xml::Deserializer, _kind: ::instant_xml::EntityType) -> Result<Self, ::instant_xml::Error> {
                println!("deserialize: {}", #name);
                use ::instant_xml::parse::XmlRecord;
                use ::instant_xml::{EntityType, Deserializer, Visitor} ;

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
                deserializer.deserialize_struct(StructVisitor{}, #name, &namespaces_map)
            }
        ));

        out.extend(quote!(
                const TAG_NAME: ::instant_xml::XMLTagName<'xml> = ::instant_xml::XMLTagName::Custom(#name);
            )
        );

        Deserializer { out }
    }

    fn process_field(
        field: &syn::Field,
        index: usize,
        declare_values: &mut TokenStream,
        return_val: &mut TokenStream,
    ) -> FieldType {
        let mut tokens = Tokens::default();
        let mut is_element = true;
        if let Some(true) = retrieve_attr("attribute", &field.attrs) {
            is_element = false
        }

        let field_name = field.ident.as_ref().unwrap().to_string();
        let const_field_name = Ident::new(&field_name.to_uppercase(), Span::call_site());
        let field_value = field.ident.as_ref().unwrap();
        let field_type = if let syn::Type::Path(v) = &field.ty {
            v.path.get_ident()
        } else {
            todo!();
        };

        let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
        tokens.enum_.extend(quote!(#enum_name,));

        tokens.consts.extend(quote!(
            const #const_field_name: &str = match #field_type::TAG_NAME {
                ::instant_xml::XMLTagName::FieldName => #field_name,
                ::instant_xml::XMLTagName::Custom(v) => v,
            };
        ));

        if is_element {
            tokens.names.extend(quote!(
                #const_field_name => __Elements::#enum_name,
            ));
        } else {
            tokens.names.extend(quote!(
                #const_field_name => __Attributes::#enum_name,
            ));
        }

        declare_values.extend(quote!(
            let mut #enum_name: Option<#field_type> = None;
        ));

        if is_element {
            tokens.match_.extend(quote!(
                __Elements::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    if item.prefix.is_some() {
                        let prefix = item.prefix.unwrap().to_string();
                        deserializer.verify_namespace(&prefix);
                    }

                    #enum_name = Some(#field_type::deserialize(deserializer, ::instant_xml::EntityType::Element)?);
                },
            ));
        } else {
            tokens.match_.extend(quote!(
                __Attributes::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    #enum_name = Some(#field_type::deserialize(deserializer, ::instant_xml::EntityType::Attribute)?);
                },
            ));
        }

        return_val.extend(quote!(
            #field_value: #enum_name.expect("Expected some value"),
        ));

        if is_element {
            FieldType::Element(tokens)
        } else {
            FieldType::Attribute(tokens)
        }
    }
}
