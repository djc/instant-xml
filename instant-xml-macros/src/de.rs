use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};

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
        let default_namespace = match &container_meta.ns.default {
            Namespace::Default => "",
            Namespace::Prefix(_) => panic!("container namespace cannot be prefix"),
            Namespace::Literal(ns) => ns,
        };

        let generics = (&input.generics).into_token_stream();
        let lifetimes = (&input.generics.params).into_token_stream();

        let mut lifetime_xml = TokenStream::new();
        let mut lifetime_visitor = TokenStream::new();
        let iter = &mut input.generics.params.iter();
        if let Some(it) = iter.next() {
            lifetime_xml = quote!(:);
            lifetime_xml.extend(it.into_token_stream());
            while let Some(it) = iter.by_ref().next() {
                lifetime_xml.extend(quote!(+));
                lifetime_xml.extend(it.into_token_stream());
            }
            lifetime_xml.extend(quote!(,));
            lifetime_xml.extend(lifetimes.clone());
            lifetime_visitor.extend(quote!(,));
            lifetime_visitor.extend(lifetimes);
        }

        let name = ident.to_string();
        let mut out = TokenStream::new();

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
                            if let Namespace::Prefix(prefix) = &field_meta.ns.default {
                                if !container_meta.ns.prefixes.contains_key(prefix) {
                                    panic!("unknown prefix for this type");
                                }
                            }

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

        out.extend(quote!(
            fn deserialize(deserializer: &mut ::instant_xml::Deserializer<'xml>) -> Result<Self, ::instant_xml::Error> {
                use ::instant_xml::de::{XmlRecord, Deserializer, Visitor};
                use ::instant_xml::Error;

                enum __Elements {
                    #elements_enum
                    __Ignore,
                }

                enum __Attributes {
                    #attributes_enum
                    __Ignore,
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
                        use ::instant_xml::de::Node;

                        #declare_values
                        while let Some(attr) = deserializer.peek_next_attribute()? {
                            let attr = {
                                #attributes_consts
                                match attr.id {
                                    #attributes_names
                                    _ => __Attributes::__Ignore
                                }
                            };

                            match attr {
                                #attr_type_match
                                __Attributes::__Ignore => {}
                            }
                        }

                        while let Some(node) = deserializer.peek_next_tag()? {
                            match node {
                                Node::Open { ns, name } => {
                                    let id = ::instant_xml::Id { ns, name };
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
                                            deserializer.ignore(id)?;
                                        }
                                    }
                                }
                                Node::Close { name } => {
                                    if name == #name {
                                        break;
                                    }
                                },
                                Node::Text { text } => panic!("Unexpected element"),
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
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #name,
            });
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

        let default_ns = match &field_meta.ns.default {
            Namespace::Default => &container_meta.ns.default,
            _ => &field_meta.ns.default,
        };

        let ns = match default_ns {
            Namespace::Default => "",
            Namespace::Prefix(prefix) => match container_meta.ns.prefixes.get(prefix) {
                Some(ns) => ns,
                None => panic!("undefined prefix {prefix} in xml attribute"),
            },
            Namespace::Literal(ns) => ns,
        };

        tokens.consts.extend(quote!(
            const #const_field_var_str: ::instant_xml::Id<'static> = <#no_lifetime_type>::KIND.name(
                ::instant_xml::Id { ns: #ns, name: #field_var_str }
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

                    #enum_name = Some(<#no_lifetime_type>::deserialize(deserializer)?);
                },
            ));
        } else {
            tokens.match_.extend(quote!(
                __Attributes::#enum_name => {
                    if #enum_name.is_some() {
                        panic!("duplicated value");
                    }

                    deserializer.set_next_type_as_attribute()?;
                    #enum_name = Some(<#no_lifetime_type>::deserialize(deserializer)?);
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
