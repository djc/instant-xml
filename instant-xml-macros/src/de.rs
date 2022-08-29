use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};

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
        let generics = (&input.generics).into_token_stream();
        let lifetimes = (&input.generics.params).into_token_stream();

        let mut lifetime_xml = TokenStream::new();
        let mut lifetime_visitor = TokenStream::new();
        let iter = &mut input.generics.params.iter();
        if let Some(it) = iter.next() {
            lifetime_xml = quote!(:);
            lifetime_xml.extend(it.into_token_stream());
            while let Some(it) = iter.by_ref().next() {
                lifetime_xml.extend(syn::token::Add::default().into_token_stream());
                lifetime_xml.extend(it.into_token_stream());
            }
            lifetime_xml.extend(quote!(,));
            lifetime_xml.extend(lifetimes.clone());
            lifetime_visitor.extend(quote!(,));
            lifetime_visitor.extend(lifetimes);
        }

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
                            let mut field_namespace = None;
                            let (tokens, def_prefix, is_element) = match retrieve_field_attribute(field) {
                                Some(FieldAttribute::Namespace(value)) => {
                                    field_namespace = Some(value);
                                    (&mut elements_tokens, None, true)
                                }
                                Some(FieldAttribute::PrefixIdentifier(def_prefix)) => {
                                    if other_namespaces.get(&def_prefix).is_none() {
                                        panic!("Namespace with such prefix do not exist for this struct");
                                    }

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
                                field_namespace,
                            );
                        });
                    }
                    syn::Fields::Unnamed(_) => panic!("unamed"),
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
            fn deserialize(deserializer: &mut ::instant_xml::Deserializer<'xml>) -> Result<Self, ::instant_xml::Error> {
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

                struct StructVisitor<'xml #lifetime_xml> {
                    marker: std::marker::PhantomData<#ident #generics>,
                    lifetime: std::marker::PhantomData<&'xml ()>,
                }

                impl<'xml #lifetime_xml> Visitor<'xml> for StructVisitor<'xml #lifetime_visitor> {
                    type Value = #ident #generics;

                    fn visit_struct(
                        &self,
                        deserializer: &mut ::instant_xml::Deserializer<'xml>
                    ) -> Result<Self::Value, ::instant_xml::Error> {
                        #declare_values
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
                                        __Elements::__Ignore => panic!("No such element"),
                                    }
                                 }
                                 XmlRecord::Close(tag) => {
                                    if tag == &#name {
                                        break;
                                    }
                                },
                                XmlRecord::Element(_) => panic!("Unexpected element"),
                            }
                        }

                        Ok(Self::Value {
                            #return_val
                    })
                    }
                }

                #namespaces_map;
                deserializer.deserialize_struct(
                    StructVisitor{
                        marker: std::marker::PhantomData,
                        lifetime: std::marker::PhantomData
                    },
                    #name,
                    #default_namespace,
                    &namespaces_map
                )
            }
        ));

        out.extend(quote!(
            const TAG_NAME: ::instant_xml::TagName<'xml> = ::instant_xml::TagName::Custom(#name);
        ));

        out = quote!(
            impl<'xml #lifetime_xml> FromXml<'xml> for #ident #generics {
                #out
            }
        );

        Deserializer { out }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_field(
        field: &syn::Field,
        index: usize,
        declare_values: &mut TokenStream,
        return_val: &mut TokenStream,
        tokens: &mut Tokens,
        is_element: bool,
        def_prefix: Option<String>,
        field_namespace: Option<String>,
    ) {
        let mut is_option_type = false;
        let field_var = field.ident.as_ref().unwrap();
        let field_var_str = field_var.to_string();
        let const_field_var_str = Ident::new(&field_var_str.to_uppercase(), Span::call_site());
        let field_type = match &field.ty {
            syn::Type::Path(v) => match v.path.get_ident() {
                Some(ident) => ident.into_token_stream(),
                None => {
                    if v.path.segments.is_empty() {
                        panic!("Wrong declaration");
                    };

                    let type_params = v.path.segments.first().expect("Struct name");
                    match type_params.ident.to_string().as_str() {
                        "Option" => match &type_params.arguments {
                            syn::PathArguments::AngleBracketed(params) => {
                                is_option_type = true;
                                let mut temp = quote!(Option<);
                                match params.args.first().unwrap() {
                                    syn::GenericArgument::Type(syn::Type::Path(v)) => {
                                        match v.path.get_ident() {
                                            Some(ident) => temp.extend(ident.into_token_stream()),
                                            None => temp.extend(
                                                (&v.path
                                                    .segments
                                                    .first()
                                                    .expect("Struct name")
                                                    .ident)
                                                    .into_token_stream(),
                                            ),
                                        }
                                    }
                                    _ => panic!("Wrong data"),
                                }

                                temp.extend(quote!(>));
                                temp
                            }
                            _ => panic!("Wrong data"),
                        },
                        _ => (&type_params.ident).into_token_stream(),
                    }
                }
            },
            syn::Type::Reference(v) => {
                let mut out = v.and_token.into_token_stream();
                out.extend((&*v.elem).into_token_stream());
                out
            }
            _ => panic!("Wrong field attribute format"),
        };

        let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
        tokens.enum_.extend(quote!(#enum_name,));

        tokens.consts.extend(quote!(
            const #const_field_var_str: &str = match <#field_type>::TAG_NAME {
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

        if is_option_type {
            declare_values.extend(quote!(
                let mut #enum_name: Option<#field_type> = Some(None);
            ));
        } else {
            declare_values.extend(quote!(
                let mut #enum_name: Option<#field_type> = None;
            ));
        }

        let def_prefix = match def_prefix {
            Some(def_prefix) => quote!(let def_prefix: Option<&str> = Some(#def_prefix);),
            None => quote!(let def_prefix: Option<&str> = None;),
        };

        let field_namespace = match field_namespace {
            Some(field_namespace) => {
                quote!(let field_namespace: Option<&str> = Some(#field_namespace);)
            }
            None => quote!(let field_namespace: Option<&str> = None;),
        };

        let duplicated_value_check = if is_option_type {
            quote!(
                if #enum_name.expect("Some value").is_some() {
                    panic!("duplicated value");
                }
            )
        } else {
            quote!(
                if #enum_name.is_some() {
                    panic!("duplicated value");
                }
            )
        };

        if is_element {
            tokens.match_.extend(quote!(
                __Elements::#enum_name => {
                    #duplicated_value_check

                    match item.prefix {
                        Some(item) => {
                            let parser_prefix = item.to_owned();
                            #def_prefix
                            match def_prefix {
                                Some(def_prefix) => {
                                    // Check if defined and gotten namespaces equals for each field
                                    if deserializer.get_parser_namespace(&parser_prefix)
                                        != deserializer.get_def_namespace(def_prefix) {
                                        return Err(Error::WrongNamespace)
                                    }
                                }
                                None => {
                                    return Err(Error::WrongNamespace);
                                }
                            }
                        }
                        None => {
                            #def_prefix
                            match def_prefix {
                                Some(_) => {
                                    return Err(Error::WrongNamespace)
                                },
                                None => (),
                            }
                        }
                    }
                    #field_namespace
                    deserializer.set_next_def_namespace(field_namespace)?;
                    #enum_name = Some(<#field_type>::deserialize(deserializer)?);
                },
            ));
        } else {
            tokens.match_.extend(quote!(
                __Attributes::#enum_name => {
                    #duplicated_value_check

                    deserializer.set_next_type_as_attribute()?;
                    #enum_name = Some(<#field_type>::deserialize(deserializer)?);
                },
            ));
        }

        return_val.extend(quote!(
            #field_var: #enum_name.expect("Expected some value"),
        ));
    }
}
