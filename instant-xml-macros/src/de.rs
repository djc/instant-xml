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
        let mut enum_fields: proc_macro2::TokenStream = TokenStream::from(quote!()).into();
        let mut field_names: proc_macro2::TokenStream = TokenStream::from(quote!()).into();

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
                                Ident::new(&format!("__field{index}"), Span::call_site());
                            let is_scalar =
                                Self::is_scalar(field_type.as_ref().unwrap().to_string().as_str());

                            enum_fields.extend(quote!(#enum_name,));

                            if !is_scalar {
                                field_name = field_type.as_ref().unwrap().to_string();
                            }

                            field_names.extend(quote!(
                                #field_name => __Field::#enum_name,
                            ));

                            declare_values.extend(quote!(
                                let mut #enum_name: Option<#field_type> = None;
                            ));

                            type_match.extend(quote!(
                                __Field::#enum_name => {
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
                D: ::instant_xml::DeserializeXml<'xml> 
            {
                println!("deserialize: {}", #name);
                use ::instant_xml::parse::XmlRecord;
                use ::instant_xml::{Deserializer, DeserializeXml, Visitor} ;

                enum __Field {
                    #enum_fields
                    __ignore,
                }

                fn get_type(value: &str) -> __Field {
                    match value {
                        #field_names
                        _ => __Field::__ignore,
                    }
                }

                struct StructVisitor;
                impl<'xml> Visitor<'xml> for StructVisitor {
                    type Value = #ident;

                    fn visit_struct<'a>(&self, deserializer: &mut ::instant_xml::Deserializer) -> Result<Self::Value, ::instant_xml::Error>
                    {
                        #declare_values
                        while let Some(item) = &deserializer.iter.next() {
                            match item {
                                XmlRecord::Open(item) => {
                                    println!("Key: {:?}", &item.key);
                                    match get_type(&item.key.as_ref().unwrap()) {
                                        #type_match
                                        __Field::__ignore => todo!(),
                                    }
                                }
                                XmlRecord::Close(tag) => {
                                    println!("Close: {}", tag); // Moze jest jakiś lepszy sposob?
                                    if tag == #name {
                                        break
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
            
                Ok(deserializer.deserialize_struct(StructVisitor{}, #name)?)
        }
        ))
        .into();

        Deserializer {
            input,
            fn_deserialize,
        }
    }

    fn is_scalar(value: &str) -> bool {
        match value {
            "bool" => true,
            "i8" => true,
            "i16" => true,
            "i32" => true,
            "i64" => true,
            "u8" => true,
            _ => false,
        }
    }
}
