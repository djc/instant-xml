use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;

pub struct Deserializer<'a> {
    /// Original input.
    pub input: &'a syn::DeriveInput,
    pub fn_deserialize: proc_macro2::TokenStream,
}

impl<'a> Deserializer<'a> {
    pub fn new(input: &syn::DeriveInput) -> Deserializer {
        let ident = &input.ident;
        let name = ident.to_string();
        let mut return_val: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut declare_values: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut type_match: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut enum_elements: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut elements_names: proc_macro2::TokenStream = TokenStream::from(quote!()).into();

        match &input.data {
            syn::Data::Struct(ref data) => {
                match data.fields {
                    syn::Fields::Named(ref fields) => {
                        fields.named.iter().enumerate().for_each(|(index, field)| {
                            let mut field_name = field.ident.as_ref().unwrap().to_string();
                            let field_value = field.ident.as_ref().unwrap();
                            let field_type = if let syn::Type::Path(v) = &field.ty {
                                v.path.get_ident()
                            } else {
                                todo!();
                            };
                            let enum_name =
                                Ident::new(&format!("__Elements{index}"), Span::call_site());
                            let is_scalar =
                                Self::is_scalar(field_type.as_ref().unwrap().to_string().as_str());

                            enum_elements.extend(quote!(#enum_name,));

                            if !is_scalar {
                                field_name = field_type.as_ref().unwrap().to_string();
                            }

                            elements_names.extend(quote!(
                                #field_name => __Elements::#enum_name,
                            ));

                            declare_values.extend(quote!(
                                let mut #enum_name: Option<#field_type> = None;
                            ));

                            type_match.extend(quote!(
                                __Elements::#enum_name => {
                                    if( #enum_name.is_some() ) {
                                        panic!("duplicated value");
                                    }
                                    #enum_name = Some(#field_type::deserialize(deserializer).unwrap());
                                },
                            ));

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

        let fn_deserialize: proc_macro2::TokenStream = TokenStream::from(quote!(
            fn deserialize<D>(deserializer: &mut D) -> Result<Self, ::instant_xml::Error>
            where
                D: ::instant_xml::DeserializeXml<'xml>,
            {
                println!("deserialize: {}", #name);
                use ::instant_xml::parse::XmlRecord;
                use ::instant_xml::{Deserializer, DeserializeXml, Visitor} ;

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

                // enum __Attributes {
                //     #enum_attributes
                //     __ignore,
                // }

                // fn get_attribute(value: &str) -> __Elements {
                //     match value {
                //         #attributes_names
                //         _ => __Elements::__ignore,
                //     }
                // }

                struct StructVisitor;
                impl<'xml> Visitor<'xml> for StructVisitor {
                    type Value = #ident;

                    fn visit_struct<'a, D>(&self, deserializer: &mut D) -> Result<Self::Value, ::instant_xml::Error>
                    where
                        D: ::instant_xml::DeserializeXml<'xml>,
                    {
                        #declare_values
                        println!("visit struct");
                        while let Some(item) = &deserializer.peek_next_tag().unwrap() {
                            match item {
                                XmlRecord::Open(item) => {
                                    match get_element(&item.key.as_ref()) {
                                        #type_match
                                        __Elements::__ignore => todo!(),
                                    }

                                    if let Some(attributes_vec) = &item.attributes {
                                        for attr in attributes_vec {
                                            // match get_attribute(&item.key.as_ref().unwrap()) {
                                            //     #type_match
                                            //     __Elements::__ignore => todo!(),
                                            // }
                                        }
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
                deserializer.deserialize_struct(StructVisitor{}, #name)
            }
        ))
        .into();

        Deserializer {
            input,
            fn_deserialize,
        }
    }

    fn is_scalar(value: &str) -> bool {
        matches!(value, "bool" | "i8" | "i16" | "i32" | "i64" | "u8") // TODO: Fill up
    }
}
