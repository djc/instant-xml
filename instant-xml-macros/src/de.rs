use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

use super::{
    discard_lifetimes, meta_items, ContainerMeta, FieldMeta, Mode, Namespace, VariantMeta,
};

pub(crate) fn from_xml(input: &syn::DeriveInput) -> TokenStream {
    let meta = match ContainerMeta::from_derive(input) {
        Ok(meta) => meta,
        Err(e) => return e.to_compile_error(),
    };

    match (&input.data, meta.mode) {
        (syn::Data::Struct(data), None) => deserialize_struct(input, data, meta),
        (syn::Data::Enum(data), Some(Mode::Scalar)) => deserialize_scalar_enum(input, data, meta),
        (syn::Data::Enum(data), Some(Mode::Wrapped)) => deserialize_wrapped_enum(input, data, meta),
        (syn::Data::Struct(_), _) => {
            syn::Error::new(input.span(), "no enum mode allowed on struct type").to_compile_error()
        }
        (syn::Data::Enum(_), None) => {
            syn::Error::new(input.span(), "missing enum mode").to_compile_error()
        }
        _ => todo!(),
    }
}

fn deserialize_scalar_enum(
    input: &syn::DeriveInput,
    data: &syn::DataEnum,
    meta: ContainerMeta,
) -> TokenStream {
    let ident = &input.ident;
    let mut variants = TokenStream::new();

    for variant in data.variants.iter() {
        let v_ident = &variant.ident;
        let meta = match VariantMeta::from_variant(variant, &meta) {
            Ok(meta) => meta,
            Err(err) => return err.to_compile_error(),
        };

        let serialize_as = meta.serialize_as;
        variants.extend(quote!(Ok(#serialize_as) => #ident::#v_ident,));
    }

    let generics = meta.xml_generics();
    let (impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    quote!(
        impl #impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            fn deserialize<'cx>(
                deserializer: &'cx mut ::instant_xml::Deserializer<'cx, 'xml>,
                into: &mut Option<Self>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::Error;

                if into.is_some() {
                    return Err(Error::DuplicateValue);
                }

                let value = match deserializer.take_str() {
                    #variants
                    _ => return Err(Error::UnexpectedValue),
                };

                *into = Some(value);
                Ok(())
            }

            const KIND: ::instant_xml::Kind<'static> = ::instant_xml::Kind::Scalar;
        }
    )
}

fn deserialize_wrapped_enum(
    input: &syn::DeriveInput,
    data: &syn::DataEnum,
    meta: ContainerMeta,
) -> TokenStream {
    if data.variants.is_empty() {
        return syn::Error::new(input.span(), "empty enum is not supported").to_compile_error();
    }

    let ident = &input.ident;
    let mut variants = TokenStream::new();
    for variant in data.variants.iter() {
        let field = match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                fields.unnamed.first().unwrap()
            }
            _ => {
                return syn::Error::new(
                    input.span(),
                    "wrapped enum variants must have 1 unnamed field",
                )
                .to_compile_error()
            }
        };

        if !meta_items(&variant.attrs).is_empty() {
            return syn::Error::new(
                input.span(),
                "attributes not allowed on wrapped enum variants",
            )
            .to_compile_error();
        }

        let mut no_lifetime_type = field.ty.clone();
        discard_lifetimes(&mut no_lifetime_type);

        if !variants.is_empty() {
            variants.extend(quote!(else));
        }

        let v_ident = &variant.ident;
        variants.extend(quote!(if <#no_lifetime_type as FromXml>::KIND.matches(
            id, ::instant_xml::Id { ns: "", name: "" }
        ) {
            let mut nested = deserializer.nested(data);
            let mut value = None;
            #no_lifetime_type::deserialize(&mut nested, &mut value)?;
            *into = value.map(#ident::#v_ident);
        }));
    }

    let name = meta.tag();
    let default_namespace = meta.default_namespace();
    let generics = meta.xml_generics();
    let (xml_impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            fn deserialize<'cx>(
                deserializer: &'cx mut ::instant_xml::Deserializer<'cx, 'xml>,
                into: &mut Option<Self>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::de::Node;
                use ::instant_xml::Error;

                let node = match deserializer.next() {
                    Some(result) => result?,
                    None => return Err(Error::MissingValue(&<Self as FromXml>::KIND)),
                };

                let data = match node {
                    Node::Open(data) => data,
                    _ => return Err(Error::UnexpectedState("unexpected node type for wrapped enum variant")),
                };

                let id = deserializer.element_id(&data)?;
                #variants else {
                    return Err(Error::UnexpectedTag);
                };

                if let Some(_) = deserializer.next() {
                    return Err(Error::UnexpectedState("unexpected node after wrapped enum variant"));
                }

                Ok(())
            }

            const KIND: ::instant_xml::Kind<'static> = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #name,
            });
        }
    )
}

fn deserialize_struct(
    input: &syn::DeriveInput,
    data: &syn::DataStruct,
    container_meta: ContainerMeta,
) -> TokenStream {
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

    match data.fields {
        syn::Fields::Named(ref fields) => {
            fields.named.iter().enumerate().for_each(|(index, field)| {
                let field_meta = match FieldMeta::from_field(field, &container_meta) {
                    Ok(meta) => meta,
                    Err(err) => {
                        return_val.extend(err.into_compile_error());
                        return;
                    }
                };

                let tokens = match field_meta.attribute {
                    true => &mut attributes_tokens,
                    false => &mut elements_tokens,
                };

                process_field(
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

    // Elements
    let elements_enum = elements_tokens.r#enum;
    let mut elements_branches = elements_tokens.branches;
    let elem_type_match = elements_tokens.r#match;
    elements_branches.extend(match elements_branches.is_empty() {
        true => quote!(__Elements::__Ignore),
        false => quote!(else { __Elements::__Ignore }),
    });

    // Attributes
    let attributes_enum = attributes_tokens.r#enum;
    let mut attributes_branches = attributes_tokens.branches;
    let attr_type_match = attributes_tokens.r#match;
    attributes_branches.extend(match attributes_branches.is_empty() {
        true => quote!(__Attributes::__Ignore),
        false => quote!(else { __Attributes::__Ignore }),
    });

    let ident = &input.ident;
    let name = container_meta.tag();
    let default_namespace = container_meta.default_namespace();
    let generics = container_meta.xml_generics();

    let (xml_impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            fn deserialize<'cx>(
                deserializer: &'cx mut ::instant_xml::Deserializer<'cx, 'xml>,
                into: &mut Option<Self>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::de::Node;
                use ::instant_xml::{Error, Id};

                enum __Elements {
                    #elements_enum
                    __Ignore,
                }

                enum __Attributes {
                    #attributes_enum
                    __Ignore,
                }

                #declare_values
                loop {
                    let node = match deserializer.next() {
                        Some(result) => result?,
                        None => break,
                    };

                    match node {
                        Node::Attribute(attr) => {
                            let id = deserializer.attribute_id(&attr)?;
                            let field = #attributes_branches;

                            match field {
                                #attr_type_match
                                __Attributes::__Ignore => {}
                            }
                        }
                        Node::Open(data) => {
                            let id = deserializer.element_id(&data)?;
                            let element = #elements_branches;

                            match element {
                                #elem_type_match
                                __Elements::__Ignore => {
                                    let mut nested = deserializer.nested(data);
                                    nested.ignore()?;
                                }
                            }
                        }
                        node => return Err(Error::UnexpectedNode(format!("{:?}", node))),
                    }
                }

                *into = Some(Self { #return_val });
                Ok(())
            }

            const KIND: ::instant_xml::Kind<'static> = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #name,
            });
        }
    )
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
    let field_name = field.ident.as_ref().unwrap();
    let field_tag = field_meta.tag;
    let default_ns = match &field_meta.ns.uri {
        None => &container_meta.ns.uri,
        _ => &field_meta.ns.uri,
    };

    let ns = match default_ns {
        Some(Namespace::Path(path)) => quote!(#path),
        Some(Namespace::Literal(ns)) => quote!(#ns),
        None => quote!(""),
    };

    let mut no_lifetime_type = field.ty.clone();
    discard_lifetimes(&mut no_lifetime_type);

    let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
    tokens.r#enum.extend(quote!(#enum_name,));

    if !tokens.branches.is_empty() {
        tokens.branches.extend(quote!(else));
    }
    tokens.branches.extend(quote!(
        if <#no_lifetime_type as FromXml>::KIND.matches(id, Id { ns: #ns, name: #field_tag })
    ));

    tokens.branches.extend(match field_meta.attribute {
        true => quote!({ __Attributes::#enum_name }),
        false => quote!({ __Elements::#enum_name }),
    });

    declare_values.extend(quote!(
        let mut #enum_name: Option<#no_lifetime_type> = None;
    ));

    if !field_meta.attribute {
        tokens.r#match.extend(quote!(
            __Elements::#enum_name => {
                let mut nested = deserializer.nested(data);
                <#no_lifetime_type>::deserialize(&mut nested, &mut #enum_name)?;
            },
        ));
    } else {
        tokens.r#match.extend(quote!(
            __Attributes::#enum_name => {
                let mut nested = deserializer.for_attr(attr);
                let new = <#no_lifetime_type>::deserialize(&mut nested, &mut #enum_name)?;
            },
        ));
    }

    return_val.extend(quote!(
        #field_name: match #enum_name {
            Some(v) => v,
            None => <#no_lifetime_type>::missing_value()?,
        },
    ));
}

#[derive(Default)]
struct Tokens {
    r#enum: TokenStream,
    branches: TokenStream,
    r#match: TokenStream,
}
