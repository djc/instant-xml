use crate::{get_namespaces, retrieve_attr};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;

pub struct Deserializer<'a> {
    /// Original input.
    pub input: &'a syn::DeriveInput,
    pub fn_deserialize: proc_macro2::TokenStream,
    pub fn_from_xml: proc_macro2::TokenStream,
}

impl<'a> Deserializer<'a> {
    pub fn new(input: &syn::DeriveInput) -> Deserializer {
        let ident = &input.ident;
        let name = ident.to_string();

        let (_, other_namespaces) = get_namespaces(&input.attrs);
        let mut namespaces_map: proc_macro2::TokenStream =
            TokenStream::from(quote!(let mut namespaces_map = std::collections::HashMap::new();))
                .into();
        for (k, v) in other_namespaces.iter() {
            namespaces_map.extend(quote!(
                namespaces_map.insert(#k, #v);
            ))
        }

        let mut return_val: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut declare_values: proc_macro2::TokenStream = TokenStream::from(quote!()).into();

        let mut type_match: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut enum_elements: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut elements_names: proc_macro2::TokenStream = TokenStream::from(quote!()).into();

        let mut enum_attributes: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut attributes_names: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut attr_type_match: proc_macro2::TokenStream = TokenStream::from(quote!()).into();

        match &input.data {
            syn::Data::Struct(ref data) => {
                match data.fields {
                    syn::Fields::Named(ref fields) => {
                        fields.named.iter().enumerate().for_each(|(index, field)| {
                            // TODO: Refactor this:
                            let mut field_name = field.ident.as_ref().unwrap().to_string();
                            let field_value = field.ident.as_ref().unwrap();
                            let field_type = if let syn::Type::Path(v) = &field.ty {
                                v.path.get_ident()
                            } else {
                                todo!();
                            };
                            let mut is_attribute = false;
                            if let Some(t) = retrieve_attr("attribute", &field.attrs) {
                                is_attribute = t;
                            }

                            let enum_name =
                                Ident::new(&format!("__Elements{index}"), Span::call_site());
                            let is_scalar =
                                Self::is_scalar(field_type.as_ref().unwrap().to_string().as_str());

                            if !is_attribute {
                                enum_elements.extend(quote!(#enum_name,));
                            } else {
                                enum_attributes.extend(quote!(#enum_name,));
                            }

                            if !is_scalar {
                                field_name = field_type.as_ref().unwrap().to_string();
                            }

                            if !is_attribute {
                                elements_names.extend(quote!(
                                    #field_name => __Elements::#enum_name,
                                ));
                            } else {
                                attributes_names.extend(quote!(
                                    #field_name => __Attributes::#enum_name,
                                ));
                            }

                            declare_values.extend(quote!(
                                let mut #enum_name: Option<#field_type> = None;
                            ));

                            if !is_attribute {
                                type_match.extend(quote!(
                                    __Elements::#enum_name => {
                                        if( #enum_name.is_some() ) {
                                            panic!("duplicated value");
                                        }
                                        #enum_name = Some(#field_type::deserialize(deserializer)?);
                                    },
                                ));
                            } else {
                                attr_type_match.extend(quote!(
                                    __Attributes::#enum_name => {
                                        if( #enum_name.is_some() ) {
                                            panic!("duplicated value");
                                        }
                                        if let Some(::instant_xml::Attribute::Value(value)) = Some(::instant_xml::Attribute::<#field_type>::deserialize(deserializer)?) {
                                            #enum_name = Some(value);
                                        }
                                    },
                                ));
                            }

                            return_val.extend(quote!(
                                #field_value: #enum_name.expect("Expected some value"),
                            ));
                        });
                    }
                    syn::Fields::Unnamed(_) => todo!(),
                    syn::Fields::Unit => {}
                };
            }
            _ => todo!(),
        };

        let fn_from_xml: proc_macro2::TokenStream = TokenStream::from(quote!(
            fn from_xml<'a>(input: &'a str) -> Result<Self, ::instant_xml::Error> {
                let mut xml_parser = ::instant_xml::parse::XmlParser::new(input);
                let mut prefixes_set = std::collections::BTreeSet::new();
                let mut current_attribute = String::new();
                let mut deserializer = ::instant_xml::Deserializer {
                    iter: &mut xml_parser,
                    prefixes: &mut prefixes_set,
                    current_attribute: &mut current_attribute,
                };
                Self::deserialize(&mut deserializer)
            }
        ))
        .into();

        let fn_deserialize: proc_macro2::TokenStream = TokenStream::from(quote!(
            fn deserialize<D>(deserializer: &mut D) -> Result<Self, ::instant_xml::Error>
            where
                D: ::instant_xml::DeserializeXml<'xml>,
            {
                println!("deserialize: {}", #name);
                use ::instant_xml::parse::XmlRecord;
                use ::instant_xml::{Deserializer, DeserializeXml, Visitor, Attribute} ;

                enum __Elements {
                    #enum_elements
                    __ignore,
                }

                fn get_element(value: &str) -> __Elements {
                    match value {
                        #elements_names
                        _ => __Elements::__ignore,
                    }
                }

                enum __Attributes {
                    #enum_attributes
                    __ignore,
                }

                fn get_attribute(value: &str) -> __Attributes {
                    match value {
                        #attributes_names
                        _ => __Attributes::__ignore,
                    }
                }

                struct StructVisitor;
                impl<'xml> Visitor<'xml> for StructVisitor {
                    type Value = #ident;

                    fn visit_struct<'a, D>(&self, deserializer: &mut D, attributes: Option<&std::collections::HashMap<String, String>>) -> Result<Self::Value, ::instant_xml::Error>
                    where
                        D: ::instant_xml::DeserializeXml<'xml> + ::instant_xml::AccessorXml<'xml>,
                    {
                        #declare_values
                        println!("visit struct");
                        while let Some(item) = &deserializer.peek_next_tag()? {
                            match item {
                                XmlRecord::Open(item) => {
                                    match get_element(&item.key.as_ref()) {
                                        #type_match
                                        __Elements::__ignore => todo!(),
                                    }

                                    // Verify prefix
                                    if let Some(prefix) = &item.prefix {
                                        // Check if such prefix exist
                                        if !deserializer.verify_prefix(&prefix) {
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

                        if let Some(attributes_map) = &attributes {
                            for (k, v) in attributes_map.iter() {
                                deserializer.set_current_attribute(v);
                                match get_attribute(&k) {
                                    #attr_type_match
                                    __Attributes::__ignore => todo!(),
                                }
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
        .into();

        Deserializer {
            input,
            fn_deserialize,
            fn_from_xml,
        }
    }

    fn is_scalar(value: &str) -> bool {
        matches!(value, "bool" | "i8" | "i16" | "i32" | "i64" | "u8") // TODO: Fill up
    }
}
