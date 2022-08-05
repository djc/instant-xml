use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::{get_namespaces, retrieve_attr};

struct Tokens<'a> {
    enum_: &'a mut TokenStream,
    consts_: &'a mut TokenStream,
    names_: &'a mut TokenStream,
    match_: &'a mut TokenStream,
}

pub struct Deserializer {
    fn_vec: Vec<TokenStream>,
}

impl Deserializer {
    pub fn new(input: &syn::DeriveInput) -> Deserializer {
        let ident = &input.ident;
        let name = ident.to_string();
        let mut fn_vec = Vec::new();

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

        // Elements
        let mut elements_enum = TokenStream::new();
        let mut elements_lets = TokenStream::new();
        let mut elements_names = TokenStream::new();
        let mut elem_type_match = TokenStream::new();
        let mut elements_tokens = Tokens {
            enum_: &mut elements_enum,
            consts_: &mut elements_lets,
            names_: &mut elements_names,
            match_: &mut elem_type_match,
        };

        // Attributes
        let mut attributes_enum = TokenStream::new();
        let mut attributes_lets = TokenStream::new();
        let mut attributes_names = TokenStream::new();
        let mut attr_type_match = TokenStream::new();
        let mut attributes_tokens = Tokens {
            enum_: &mut attributes_enum,
            consts_: &mut attributes_lets,
            names_: &mut attributes_names,
            match_: &mut attr_type_match,
        };

        // Common values
        let mut declare_values = TokenStream::new();
        let mut return_val = TokenStream::new();

        match &input.data {
            syn::Data::Struct(ref data) => {
                match data.fields {
                    syn::Fields::Named(ref fields) => {
                        fields.named.iter().enumerate().for_each(|(index, field)| {
                            if let Some(true) = retrieve_attr("attribute", &field.attrs) {
                                Self::process_field(
                                    field,
                                    index,
                                    &mut attributes_tokens,
                                    &mut declare_values,
                                    &mut return_val,
                                    false,
                                );
                            } else {
                                Self::process_field(
                                    field,
                                    index,
                                    &mut elements_tokens,
                                    &mut declare_values,
                                    &mut return_val,
                                    true,
                                );
                            }
                        });
                    }
                    syn::Fields::Unnamed(_) => todo!(),
                    syn::Fields::Unit => {}
                };
            }
            _ => todo!(),
        };

        fn_vec.push(
            proc_macro::TokenStream::from(quote!(
                fn from_xml<'a>(input: &'a str) -> Result<Self, ::instant_xml::Error> {
                    let mut deserializer = ::instant_xml::Deserializer::new(input);
                    Self::deserialize(&mut deserializer, ::instant_xml::EntityType::Element)
                }
            ))
            .into(),
        );

        fn_vec.push(proc_macro::TokenStream::from(quote!(
            fn deserialize(deserializer: &mut ::instant_xml::Deserializer, _kind: ::instant_xml::EntityType) -> Result<Self, ::instant_xml::Error> {
                println!("deserialize: {}", #name);
                use ::instant_xml::parse::XmlRecord;
                use ::instant_xml::{EntityType, Deserializer, Visitor} ;

                enum __Elements {
                    #elements_enum
                    __Ignore,
                }

                fn get_element(value: &str) -> __Elements {
                    #elements_lets
                    #elements_names
                    __Elements::__Ignore
                }

                enum __Attributes {
                    #attributes_enum
                    __Ignore,
                }

                fn get_attribute(value: &str) -> __Attributes {
                    #attributes_lets
                    #attributes_names
                    __Attributes::__Ignore
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

                                    // Verify prefix
                                    if let Some(prefix) = &item.prefix {
                                        // Check if such prefix exist
                                        if !deserializer.verify_namespace(&prefix) {
                                            return Err(::instant_xml::Error::UnexpectedPrefix);
                                        }
                                        // TODO: Check if prefix is equel to declared prefix
                                    }
                                 }
                                 XmlRecord::Close(tag) => {
                                    println!("Close: {}", tag);
                                    if tag == #name {
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
        ))
        .into());

        fn_vec.push(
            proc_macro::TokenStream::from(quote!(
                const TAG_NAME: ::instant_xml::XMLTagName<'xml> = ::instant_xml::XMLTagName::Custom(#name);
            ))
            .into(),
        );

        Deserializer { fn_vec }
    }

    pub fn fn_vec(&self) -> &Vec<TokenStream> {
        &self.fn_vec
    }

    fn process_field(
        field: &syn::Field,
        index: usize,
        tokens: &mut Tokens,
        declare_values: &mut TokenStream,
        return_val: &mut TokenStream,
        is_element: bool,
    ) {
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

        tokens.consts_.extend(quote!(
            const #const_field_name: &str = match #field_type::TAG_NAME {
                ::instant_xml::XMLTagName::FieldName => #field_name,
                ::instant_xml::XMLTagName::Custom(v) => v,
            };
        ));

        if is_element {
            tokens.names_.extend(quote!(
                if( value == #const_field_name ) {
                    return __Elements::#enum_name;
                };
            ));
        } else {
            tokens.names_.extend(quote!(
                if( value == #const_field_name ) {
                    return __Attributes::#enum_name;
                };
            ));
        }

        declare_values.extend(quote!(
            let mut #enum_name: Option<#field_type> = None;
        ));

        if is_element {
            tokens.match_.extend(quote!(
                __Elements::#enum_name => {
                    if( #enum_name.is_some() ) {
                        panic!("duplicated value");
                    }
                    #enum_name = Some(#field_type::deserialize(deserializer, ::instant_xml::EntityType::Element)?);
                },
            ));
        } else {
            tokens.match_.extend(quote!(
                __Attributes::#enum_name => {
                    if( #enum_name.is_some() ) {
                        panic!("duplicated value");
                    }
                    #enum_name = Some(#field_type::deserialize(deserializer, ::instant_xml::EntityType::Attribute)?);
                },
            ));
        }

        return_val.extend(quote!(
            #field_value: #enum_name.expect("Expected some value"),
        ));
    }
}
