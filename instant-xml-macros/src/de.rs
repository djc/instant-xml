use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

use super::{discard_lifetimes, ContainerMeta, FieldMeta, Namespace, VariantMeta};

pub(crate) fn from_xml(input: &syn::DeriveInput) -> TokenStream {
    let ident = &input.ident;
    let meta = ContainerMeta::from_derive(input);

    match &input.data {
        syn::Data::Struct(_) if meta.scalar => {
            syn::Error::new(input.span(), "scalar structs are unsupported!").to_compile_error()
        }
        syn::Data::Struct(ref data) => deserialize_struct(input, data, meta, ident),
        syn::Data::Enum(_) if !meta.scalar => {
            syn::Error::new(input.span(), "non-scalar enums are currently unsupported!")
                .to_compile_error()
        }
        syn::Data::Enum(ref data) => deserialize_enum(input, data, meta),
        _ => todo!(),
    }
}

#[rustfmt::skip]
fn deserialize_enum(input: &syn::DeriveInput, data: &syn::DataEnum, meta: ContainerMeta) -> TokenStream {
    let ident = &input.ident;
    let mut variants = TokenStream::new();

    for variant in data.variants.iter() {
	let v_ident = &variant.ident;
        let meta = match VariantMeta::from_variant(variant, &meta) {
	    Ok(meta) => meta,
	    Err(err) => return err.to_compile_error()
	};

        let serialize_as = meta.serialize_as;
        variants.extend(quote!(Ok(#serialize_as) => #ident::#v_ident,));
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote!(
	impl #impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            fn deserialize<'cx>(deserializer: &'cx mut ::instant_xml::Deserializer<'cx, 'xml>) -> Result<Self, ::instant_xml::Error> {
		match deserializer.take_str() {
		    #variants
		    _ => Err(::instant_xml::Error::UnexpectedValue)
		}
	    }
	}
    )
}

fn deserialize_struct(
    input: &syn::DeriveInput,
    data: &syn::DataStruct,
    container_meta: ContainerMeta,
    ident: &Ident,
) -> TokenStream {
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
    let (xml_impl_generics, _, _) = xml_generics.split_for_impl();

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
                let field_meta = match FieldMeta::from_field(field) {
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
    let elements_consts = elements_tokens.consts;
    let elements_names = elements_tokens.names;
    let elem_type_match = elements_tokens.r#match;

    // Attributes
    let attributes_enum = attributes_tokens.r#enum;
    let attributes_consts = attributes_tokens.consts;
    let attributes_names = attributes_tokens.names;
    let attr_type_match = attributes_tokens.r#match;

    let name = match &container_meta.rename {
        Some(name) => quote!(#name),
        None => ident.to_string().into_token_stream(),
    };

    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            fn deserialize<'cx>(deserializer: &'cx mut ::instant_xml::Deserializer<'cx, 'xml>) -> Result<Self, ::instant_xml::Error> {
                use ::instant_xml::de::{Deserializer, Node};
                use ::instant_xml::{Error, Id};
                use ::core::marker::PhantomData;

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

                Ok(Self { #return_val })
            }

            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element(::instant_xml::Id {
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

    let field_tag = match &field_meta.rename {
        Some(rename) => quote!(#rename),
        None => container_meta
            .rename_all
            .apply_to_field(&field_name.to_string())
            .into_token_stream(),
    };

    let const_field_var_str = Ident::new(&field_name.to_string().to_uppercase(), Span::call_site());
    let mut no_lifetime_type = field.ty.clone();
    discard_lifetimes(&mut no_lifetime_type);

    let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
    tokens.r#enum.extend(quote!(#enum_name,));

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
        const #const_field_var_str: Id<'static> = <#no_lifetime_type as FromXml<'_>>::KIND.name(
            Id { ns: #ns, name: #field_tag }
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
        tokens.r#match.extend(quote!(
            __Elements::#enum_name => {
                if #enum_name.is_some() {
                    return Err(Error::DuplicateValue);
                }

                let mut nested = deserializer.nested(data);
                #enum_name = Some(<#no_lifetime_type>::deserialize(&mut nested)?);
            },
        ));
    } else {
        tokens.r#match.extend(quote!(
            __Attributes::#enum_name => {
                if #enum_name.is_some() {
                    return Err(Error::DuplicateValue);
                }

                let mut nested = deserializer.for_attr(attr);
                #enum_name = Some(<#no_lifetime_type>::deserialize(&mut nested)?);
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

struct Tokens {
    r#enum: TokenStream,
    consts: TokenStream,
    names: TokenStream,
    r#match: TokenStream,
}

impl Default for Tokens {
    fn default() -> Self {
        Self {
            r#enum: TokenStream::new(),
            consts: TokenStream::new(),
            names: TokenStream::new(),
            r#match: TokenStream::new(),
        }
    }
}
