use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::{ContainerMeta, FieldMeta, Namespace};

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
        let container_meta = ContainerMeta::from_derive(input);
        let default_namespace = match &container_meta.ns.uri {
            Some(ns) => quote!(#ns),
            None => quote!(""),
        };

        let mut xml_generics = input.generics.clone();
        let mut xml = syn::LifetimeDef::new(syn::Lifetime::new("'xml", Span::call_site()));
        xml.bounds
            .extend(xml_generics.lifetimes().map(|lt| lt.lifetime.clone()));
        xml_generics.params.push(xml.into());

        let (_, ty_generics, where_clause) = input.generics.split_for_impl();
        let (xml_impl_generics, xml_ty_generics, xml_where_clause) = xml_generics.split_for_impl();

        let mut namespaces_map = quote!(let mut namespaces_map = std::collections::HashMap::new(););
        for (k, v) in container_meta.ns.prefixes.iter() {
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
                            let field_meta = FieldMeta::from_field(field);
                            let tokens = match field_meta.attribute {
                                true => &mut attributes_tokens,
                                false => &mut elements_tokens,
                            };

                            Self::process_field(
                                field,
                                index,
                                &mut declare_values,
                                &mut return_val,
                                tokens,
                                field_meta,
                                &container_meta,
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

        let name = ident.to_string();
        let mut out = TokenStream::new();
        out.extend(quote!(
            fn deserialize<'cx>(deserializer: &'cx mut ::instant_xml::Deserializer<'cx, 'xml>) -> Result<Self, ::instant_xml::Error> {
                use ::instant_xml::de::{Deserializer, Id, Visitor, Node};
                use ::instant_xml::Error;
                use ::core::marker::PhantomData;

                enum __Elements {
                    #elements_enum
                    __Ignore,
                }

                enum __Attributes {
                    #attributes_enum
                    __Ignore,
                }

                struct StructVisitor #xml_ty_generics {
                    marker: PhantomData<#ident #ty_generics>,
                    lifetime: PhantomData<&'xml ()>,
                }

                impl #xml_impl_generics Visitor<'xml> for StructVisitor #xml_ty_generics #xml_where_clause {
                    type Value = #ident #ty_generics;

                    fn visit_struct<'cx>(
                        deserializer: &'cx mut Deserializer<'cx, 'xml>,
                    ) -> Result<Self::Value, Error> {
                        #declare_values
                        loop {
                            let node = match deserializer.next() {
                                Some(result) => result?,
                                None => break,
                            };

                            match node {
                                Node::Attribute(attr) => {
                                    let id = deserializer.attribute_id(&attr)?;
                                    let field = {
                                        #attributes_consts
                                        match id {
                                            #attributes_names
                                            _ => __Attributes::__Ignore
                                        }
                                    };

                                    match field {
                                        #attr_type_match
                                        __Attributes::__Ignore => {}
                                    }
                                }
                                Node::Open(data) => {
                                    let id = deserializer.element_id(&data)?;
                                    let element = {
                                        #elements_consts
                                        match id {
                                            #elements_names
                                            _ => __Elements::__Ignore
                                        }
                                    };

                                    match element {
                                        #elem_type_match
                                        __Elements::__Ignore => {
                                            let mut nested = deserializer.nested(data);
                                            nested.ignore()?;
                                        }
                                    }
                                }
                                _ => return Err(Error::UnexpectedState),
                            }
                        }

                        Ok(Self::Value {
                            #return_val
                        })
                    }
                }

                StructVisitor::visit_struct(deserializer)
            }
        ));

        out.extend(quote!(
            const KIND: ::instant_xml::de::Kind = ::instant_xml::de::Kind::Element(::instant_xml::de::Id {
                ns: #default_namespace,
                name: #name,
            });
        ));

        out = quote!(
            impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
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
        field_meta: FieldMeta,
        container_meta: &ContainerMeta,
    ) {
        let field_var = field.ident.as_ref().unwrap();
        let field_var_str = field_var.to_string();
        let const_field_var_str = Ident::new(&field_var_str.to_uppercase(), Span::call_site());
        let mut no_lifetime_type = field.ty.clone();
        discard_lifetimes(&mut no_lifetime_type);

        let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
        tokens.enum_.extend(quote!(#enum_name,));

        let default_ns = match &field_meta.ns.uri {
            None => &container_meta.ns.uri,
            _ => &field_meta.ns.uri,
        };

        let ns = match default_ns {
            Some(Namespace::Path(path)) => quote!(#path),
            Some(Namespace::Literal(ns)) => quote!(#ns),
            None => quote!(""),
        };

        tokens.consts.extend(quote!(
            const #const_field_var_str: Id<'static> = <#no_lifetime_type>::KIND.name(
                Id { ns: #ns, name: #field_var_str }
            );
        ));

        if !field_meta.attribute {
            tokens.names.extend(quote!(
                #const_field_var_str => __Elements::#enum_name,
            ));
        } else {
            tokens.names.extend(quote!(
                #const_field_var_str => __Attributes::#enum_name,
            ));
        }

        declare_values.extend(quote!(
            let mut #enum_name: Option<#no_lifetime_type> = None;
        ));

        if !field_meta.attribute {
            tokens.match_.extend(quote!(
                __Elements::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    let mut nested = deserializer.nested(data);
                    #enum_name = Some(<#no_lifetime_type>::deserialize(&mut nested)?);
                },
            ));
        } else {
            tokens.match_.extend(quote!(
                __Attributes::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    let mut nested = deserializer.for_attr(attr);
                    #enum_name = Some(<#no_lifetime_type>::deserialize(&mut nested)?);
                },
            ));
        }

        return_val.extend(quote!(
            #field_var: match #enum_name {
                Some(v) => v,
                None => <#no_lifetime_type>::missing_value()?,
            },
        ));
    }
}

fn discard_lifetimes(ty: &mut syn::Type) {
    match ty {
        syn::Type::Path(ty) => discard_path_lifetimes(ty),
        syn::Type::Reference(ty) => {
            ty.lifetime = None;
            discard_lifetimes(&mut ty.elem);
        }
        _ => {}
    }
}

fn discard_path_lifetimes(path: &mut syn::TypePath) {
    if let Some(q) = &mut path.qself {
        discard_lifetimes(&mut q.ty);
    }

    for segment in &mut path.path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(args) => {
                args.args.iter_mut().for_each(|arg| match arg {
                    syn::GenericArgument::Lifetime(lt) => {
                        *lt = syn::Lifetime::new("'_", Span::call_site())
                    }
                    syn::GenericArgument::Type(ty) => discard_lifetimes(ty),
                    syn::GenericArgument::Binding(_)
                    | syn::GenericArgument::Constraint(_)
                    | syn::GenericArgument::Const(_) => {}
                })
            }
            syn::PathArguments::Parenthesized(args) => {
                args.inputs.iter_mut().for_each(discard_lifetimes)
            }
        }
    }
}
