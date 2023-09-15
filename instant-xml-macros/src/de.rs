use std::collections::BTreeSet;

use proc_macro2::{Ident, Literal, Span, TokenStream};
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
        (syn::Data::Struct(data), None) => match &data.fields {
            syn::Fields::Named(fields) => deserialize_struct(input, fields, meta),
            syn::Fields::Unnamed(fields) => deserialize_tuple_struct(input, fields, meta),
            syn::Fields::Unit => deserialize_unit_struct(input, &meta),
        },
        (syn::Data::Struct(data), Some(Mode::Transparent)) => match &data.fields {
            syn::Fields::Named(fields) => deserialize_inline_struct(input, fields, meta),
            _ => syn::Error::new(
                input.span(),
                "inline mode is only supported on types with named fields",
            )
            .to_compile_error(),
        },
        (syn::Data::Enum(data), Some(Mode::Scalar)) => deserialize_scalar_enum(input, data, meta),
        (syn::Data::Enum(data), Some(Mode::Forward)) => deserialize_forward_enum(input, data, meta),
        (syn::Data::Struct(_), Some(mode)) => syn::Error::new(
            input.span(),
            format_args!("{mode:?} mode not allowed on struct type"),
        )
        .to_compile_error(),
        (syn::Data::Enum(_), Some(mode)) => syn::Error::new(
            input.span(),
            format_args!("{mode:?} mode not allowed on enum type"),
        )
        .to_compile_error(),
        (syn::Data::Enum(_), None) => {
            syn::Error::new(input.span(), "missing mode").to_compile_error()
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
        variants.extend(quote!(Some(#serialize_as) => #ident::#v_ident,));
    }

    let generics = meta.xml_generics(BTreeSet::new());
    let (impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_str = ident.to_string();

    quote!(
        impl #impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            #[inline]
            fn matches(id: ::instant_xml::Id<'_>, field: Option<::instant_xml::Id<'_>>) -> bool {
                match field {
                    Some(field) => id == field,
                    None => false,
                }
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::Error;

                if into.is_some() {
                    return Err(Error::DuplicateValue);
                }

                let value = match deserializer.take_str()? {
                    #variants
                    Some(val) => return Err(Error::UnexpectedValue(
                        format!("enum variant not found for '{}'", val)
                    )),
                    None => return Err(Error::MissingValue(#type_str)),
                };

                *into = Some(value);
                Ok(())
            }

            type Accumulator = Option<Self>;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Scalar;
        }
    )
}

fn deserialize_forward_enum(
    input: &syn::DeriveInput,
    data: &syn::DataEnum,
    meta: ContainerMeta,
) -> TokenStream {
    if data.variants.is_empty() {
        return syn::Error::new(input.span(), "empty enum is not supported").to_compile_error();
    }

    let ident = &input.ident;
    let field_str = format!("{ident}::0");
    let mut matches = TokenStream::new();
    let mut variants = TokenStream::new();
    let mut borrowed = BTreeSet::new();
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
        discard_lifetimes(&mut no_lifetime_type, &mut borrowed, false, true);

        if !matches.is_empty() {
            matches.extend(quote!(||));
        }
        matches.extend(quote!(<#no_lifetime_type as FromXml>::matches(id, field)));

        if !variants.is_empty() {
            variants.extend(quote!(else));
        }

        let v_ident = &variant.ident;
        variants.extend(
            quote!(if <#no_lifetime_type as FromXml>::matches(id, None) {
                let mut value = None;
                <#no_lifetime_type as FromXml>::deserialize(&mut value, #field_str, deserializer)?;
                *into = value.map(#ident::#v_ident);
            }),
        );
    }

    let generics = meta.xml_generics(borrowed);
    let (xml_impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            #[inline]
            fn matches(id: ::instant_xml::Id<'_>, field: Option<::instant_xml::Id<'_>>) -> bool {
                use ::instant_xml::FromXml;
                #matches
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::de::Node;
                use ::instant_xml::{Accumulate, Error, FromXml};

                let id = deserializer.parent();
                #variants else {
                    return Err(Error::UnexpectedTag(format!("{:?}", id)));
                };

                if let Some(_) = deserializer.next() {
                    return Err(Error::UnexpectedState("unexpected node after wrapped enum variant"));
                }

                Ok(())
            }

            type Accumulator = Option<Self>;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element;
        }
    )
}

fn deserialize_struct(
    input: &syn::DeriveInput,
    fields: &syn::FieldsNamed,
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
    let mut direct = TokenStream::new();

    let mut borrowed = BTreeSet::new();
    for (index, field) in fields.named.iter().enumerate() {
        if !direct.is_empty() {
            return syn::Error::new(field.span(), "direct field must be the last")
                .into_compile_error();
        }

        let field_meta = match FieldMeta::from_field(field, &container_meta) {
            Ok(meta) => meta,
            Err(err) => return err.into_compile_error(),
        };

        let tokens = match field_meta.attribute {
            true => &mut attributes_tokens,
            false => &mut elements_tokens,
        };

        let result = named_field(
            field,
            index,
            &mut declare_values,
            &mut return_val,
            tokens,
            &mut borrowed,
            &mut direct,
            field_meta,
            &input.ident,
            &container_meta,
        );

        if let Err(err) = result {
            return err.into_compile_error();
        }
    }

    if direct.is_empty() {
        direct.extend(quote!(Node::Text(_) => {
            // no direct field, ignore
        }));
    }

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
    let ident_str = format!("{ident}");
    let name = container_meta.tag();
    let default_namespace = container_meta.default_namespace();
    let generics = container_meta.xml_generics(borrowed);

    let (xml_impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            #[inline]
            fn matches(id: ::instant_xml::Id<'_>, field: Option<::instant_xml::Id<'_>>) -> bool {
                id == ::instant_xml::Id { ns: #default_namespace, name: #name }
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::de::Node;
                use ::instant_xml::{Accumulate, Error, FromXml, Id, Kind};

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
                        #direct
                        node => return Err(Error::UnexpectedNode(format!("{:?} in {}", node, #ident_str))),
                    }
                }

                *into = Some(Self { #return_val });
                Ok(())
            }

            type Accumulator = Option<Self>;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element;
        }
    )
}

fn deserialize_inline_struct(
    input: &syn::DeriveInput,
    fields: &syn::FieldsNamed,
    meta: ContainerMeta,
) -> TokenStream {
    if !meta.ns.prefixes.is_empty() {
        return syn::Error::new(
            input.span(),
            "inline structs cannot have namespace declarations",
        )
        .to_compile_error();
    } else if let Some(ns) = meta.ns.uri {
        return syn::Error::new(
            ns.span(),
            "inline structs cannot have namespace declarations",
        )
        .to_compile_error();
    } else if let Some(rename) = meta.rename {
        return syn::Error::new(rename.span(), "inline structs cannot be renamed")
            .to_compile_error();
    }

    // Varying values
    let mut elements_tokens = Tokens::default();

    // Common values
    let mut declare_values = TokenStream::new();
    let mut return_val = TokenStream::new();
    let mut direct = TokenStream::new();

    let mut borrowed = BTreeSet::new();
    let mut matches = TokenStream::new();
    let mut acc_field_defs = TokenStream::new();
    let mut acc_field_inits = TokenStream::new();
    let mut deserialize = TokenStream::new();
    let mut acc_field_defaults = TokenStream::new();
    for (index, field) in fields.named.iter().enumerate() {
        let field_meta = match FieldMeta::from_field(field, &meta) {
            Ok(meta) => meta,
            Err(err) => return err.into_compile_error(),
        };

        if field_meta.direct {
            return syn::Error::new(field.span(), "inline structs cannot have a direct field")
                .to_compile_error();
        } else if field_meta.attribute {
            return syn::Error::new(field.span(), "inline structs cannot have attribute fields")
                .to_compile_error();
        }

        let result = named_field(
            field,
            index,
            &mut declare_values,
            &mut return_val,
            &mut elements_tokens,
            &mut borrowed,
            &mut direct,
            field_meta,
            &input.ident,
            &meta,
        );

        let data = match result {
            Ok(data) => data,
            Err(err) => return err.into_compile_error(),
        };

        if !matches.is_empty() {
            matches.extend(quote!(||));
        }

        let field_ty = data.no_lifetime_type;
        matches.extend(quote!(
            <#field_ty as FromXml<'xml>>::matches(id, None)
        ));

        let field_name = &field.ident;
        let field_ty_with_lifetime = &field.ty;
        acc_field_defs
            .extend(quote!(#field_name: <#field_ty_with_lifetime as FromXml<'xml>>::Accumulator,));
        let field_str = format!("{}::{}", input.ident, data.field_name);
        acc_field_inits.extend(quote!(#field_name: self.#field_name.try_done(#field_str)?,));
        acc_field_defaults.extend(quote!(#field_name: Default::default(),));

        if !deserialize.is_empty() {
            deserialize.extend(quote!(else));
        }
        if let Some(with) = data.deserialize_with {
            deserialize.extend(
                quote!(if <#field_ty as FromXml<'xml>>::matches(current, None) {
                    #with(&mut into.#field_name, #field_str, deserializer)?;
                }),
            );
        } else {
            deserialize.extend(quote!(if <#field_ty as FromXml<'xml>>::matches(current, None) {
                match <#field_ty as FromXml>::KIND {
                    Kind::Element => {
                        <#field_ty>::deserialize(&mut into.#field_name, #field_str, deserializer)?;
                    }
                    Kind::Scalar => {
                        <#field_ty>::deserialize(&mut into.#field_name, #field_str, deserializer)?;
                        deserializer.ignore()?;
                    }
                }
            }));
        }
    }

    // Attributes
    let ident = &input.ident;
    let accumulator = Ident::new(&format!("__{}Accumulator", ident), Span::call_site());
    let generics = meta.xml_generics(borrowed);

    let (xml_impl_generics, xml_ty_generics, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let visibility = &input.vis;

    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            #[inline]
            fn matches(id: ::instant_xml::Id<'_>, _: Option<::instant_xml::Id<'_>>) -> bool {
                #matches
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                _: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::Kind;

                let current = deserializer.parent();
                #deserialize

                Ok(())
            }

            type Accumulator = #accumulator #xml_ty_generics;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element;
        }

        #visibility struct #accumulator #xml_ty_generics #where_clause {
            #acc_field_defs
        }

        impl #xml_impl_generics ::instant_xml::Accumulate<#ident #ty_generics> for #accumulator #xml_ty_generics #where_clause {
            fn try_done(self, _: &'static str) -> Result<#ident #ty_generics, ::instant_xml::Error> {
                Ok(#ident {
                    #acc_field_inits
                })
            }
        }

        impl #xml_impl_generics Default for #accumulator #xml_ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #acc_field_defaults
                }
            }
        }
    )
}

#[allow(clippy::too_many_arguments)]
fn named_field<'a>(
    field: &'a syn::Field,
    index: usize,
    declare_values: &mut TokenStream,
    return_val: &mut TokenStream,
    tokens: &mut Tokens,
    borrowed: &mut BTreeSet<syn::Lifetime>,
    direct: &mut TokenStream,
    mut field_meta: FieldMeta,
    type_name: &Ident,
    container_meta: &ContainerMeta,
) -> Result<FieldData<'a>, syn::Error> {
    let field_name = field.ident.as_ref().unwrap();
    let field_tag = field_meta.tag;
    let default_ns = match &field_meta.ns.uri {
        None if field_meta.attribute => &None,
        None => &container_meta.ns.uri,
        _ => &field_meta.ns.uri,
    };

    let ns = match default_ns {
        Some(Namespace::Path(path)) => quote!(#path),
        Some(Namespace::Literal(ns)) => quote!(#ns),
        None => quote!(""),
    };

    if field_meta.borrow && field_meta.deserialize_with.is_none() {
        if is_cow(&field.ty, is_str) {
            field_meta.deserialize_with =
                Some(Literal::string("::instant_xml::de::borrow_cow_str"));
        } else if is_cow(&field.ty, is_slice_u8) {
            field_meta.deserialize_with =
                Some(Literal::string("::instant_xml::de::borrow_cow_slice_u8"));
        }
    }

    let mut no_lifetime_type = field.ty.clone();
    discard_lifetimes(&mut no_lifetime_type, borrowed, field_meta.borrow, true);

    let enum_name = Ident::new(&format!("__Value{index}"), Span::call_site());
    if !field_meta.direct {
        tokens.r#enum.extend(quote!(#enum_name,));

        if !tokens.branches.is_empty() {
            tokens.branches.extend(quote!(else));
        }
        tokens.branches.extend(quote!(
            if <#no_lifetime_type as FromXml>::matches(id, Some(Id { ns: #ns, name: #field_tag }))
        ));

        tokens.branches.extend(match field_meta.attribute {
            true => quote!({ __Attributes::#enum_name }),
            false => quote!({ __Elements::#enum_name }),
        });
    }

    let val_name = Ident::new(&format!("__value{index}"), Span::call_site());
    declare_values.extend(quote!(
        let mut #val_name = <#no_lifetime_type as FromXml>::Accumulator::default();
    ));

    let deserialize_with = field_meta
        .deserialize_with
        .map(|with| {
            let path = with.to_string();
            syn::parse_str::<syn::Path>(path.trim_matches('"')).map_err(|err| {
                syn::Error::new(
                    with.span(),
                    format!("failed to parse deserialize_with as path: {err}"),
                )
            })
        })
        .transpose()?;

    let field_str = format!("{type_name}::{field_name}");
    if !field_meta.attribute {
        if let Some(with) = &deserialize_with {
            if field_meta.direct {
                return Err(syn::Error::new(
                    field.span(),
                    "direct attribute is not supported deserialization functions",
                ));
            }

            tokens.r#match.extend(quote!(
                __Elements::#enum_name => {
                    let mut nested = deserializer.nested(data);
                    #with(&mut #val_name, #field_str, &mut nested)?;
                },
            ));
        } else if field_meta.direct {
            direct.extend(quote!(
                Node::Text(text) => {
                    let mut nested = deserializer.for_node(Node::Text(text));
                    <#no_lifetime_type>::deserialize(&mut #val_name, #field_str, &mut nested)?;
                }
            ));
        } else {
            tokens.r#match.extend(quote!(
                __Elements::#enum_name => match <#no_lifetime_type as FromXml>::KIND {
                    Kind::Element => {
                        let mut nested = deserializer.nested(data);
                        <#no_lifetime_type>::deserialize(&mut #val_name, #field_str, &mut nested)?;
                    }
                    Kind::Scalar => {
                        let mut nested = deserializer.nested(data);
                        <#no_lifetime_type>::deserialize(&mut #val_name, #field_str, &mut nested)?;
                        nested.ignore()?;
                    }
                },
            ));
        }
    } else {
        if field_meta.direct {
            return Err(syn::Error::new(
                field.span(),
                "direct attribute is not supported for attribute fields",
            ));
        }

        if let Some(with) = &deserialize_with {
            tokens.r#match.extend(quote!(
                __Attributes::#enum_name => {
                    let mut nested = deserializer.nested(data);
                    #with(&mut #val_name, #field_str, &mut nested)?;
                },
            ));
        } else {
            tokens.r#match.extend(quote!(
                __Attributes::#enum_name => {
                    let mut nested = deserializer.for_node(Node::AttributeValue(attr.value));
                    let new = <#no_lifetime_type as FromXml>::deserialize(&mut #val_name, #field_str, &mut nested)?;
                },
            ));
        }
    };

    return_val.extend(quote!(
        #field_name: #val_name.try_done(#field_str)?,
    ));

    Ok(FieldData {
        field_name,
        no_lifetime_type,
        deserialize_with,
    })
}

struct FieldData<'a> {
    field_name: &'a Ident,
    no_lifetime_type: syn::Type,
    deserialize_with: Option<syn::Path>,
}

fn deserialize_tuple_struct(
    input: &syn::DeriveInput,
    fields: &syn::FieldsUnnamed,
    container_meta: ContainerMeta,
) -> TokenStream {
    let mut namespaces_map = quote!(let mut namespaces_map = std::collections::HashMap::new(););
    for (k, v) in container_meta.ns.prefixes.iter() {
        namespaces_map.extend(quote!(
            namespaces_map.insert(#k, #v);
        ))
    }

    // Varying values
    let mut declare_values = TokenStream::new();
    let mut return_val = TokenStream::new();
    let mut borrowed = BTreeSet::new();
    for (index, field) in fields.unnamed.iter().enumerate() {
        if !field.attrs.is_empty() {
            return syn::Error::new(
                field.span(),
                "attributes not allowed on tuple struct fields",
            )
            .to_compile_error();
        }

        unnamed_field(
            field,
            index,
            &mut declare_values,
            &mut return_val,
            &mut borrowed,
            &input.ident,
        );
    }

    let ident = &input.ident;
    let name = container_meta.tag();
    let default_namespace = container_meta.default_namespace();
    let generics = container_meta.xml_generics(borrowed);

    let (xml_impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            #[inline]
            fn matches(id: ::instant_xml::Id<'_>, field: Option<::instant_xml::Id<'_>>) -> bool {
                id == ::instant_xml::Id { ns: #default_namespace, name: #name }
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                use ::instant_xml::de::Node;
                use ::instant_xml::{Accumulate, Error, FromXml, Id, Kind};

                #declare_values
                deserializer.ignore()?;

                *into = Some(Self(#return_val));
                Ok(())
            }

            type Accumulator = Option<Self>;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element;
        }
    )
}

#[allow(clippy::too_many_arguments)]
fn unnamed_field(
    field: &syn::Field,
    index: usize,
    declare_values: &mut TokenStream,
    return_val: &mut TokenStream,
    borrowed: &mut BTreeSet<syn::Lifetime>,
    type_name: &Ident,
) {
    let mut no_lifetime_type = field.ty.clone();
    discard_lifetimes(&mut no_lifetime_type, borrowed, false, true);

    let name = Ident::new(&format!("v{index}"), Span::call_site());
    let field_str = format!("{type_name}::{index}");
    declare_values.extend(quote!(
        let #name = match <#no_lifetime_type as FromXml>::KIND {
            Kind::Element => match deserializer.next() {
                Some(Ok(Node::Open(data))) => {
                    let mut nested = deserializer.nested(data);
                    let mut value = <#no_lifetime_type as FromXml>::Accumulator::default();
                    <#no_lifetime_type as FromXml>::deserialize(&mut value, #field_str, &mut nested)?;
                    nested.ignore()?;
                    value
                }
                Some(Ok(node)) => return Err(Error::UnexpectedNode(format!("{:?}", node))),
                Some(Err(e)) => return Err(e),
                None => return Err(Error::MissingValue(#field_str)),
            }
            Kind::Scalar => {
                let mut value = <#no_lifetime_type as FromXml>::Accumulator::default();
                <#no_lifetime_type as FromXml>::deserialize(&mut value, #field_str, deserializer)?;
                value
            }
        };
    ));

    return_val.extend(quote!(
        #name.try_done(#field_str)?,
    ));
}

fn deserialize_unit_struct(input: &syn::DeriveInput, meta: &ContainerMeta) -> TokenStream {
    let ident = &input.ident;
    let name = meta.tag();
    let default_namespace = meta.default_namespace();
    let generics = meta.xml_generics(BTreeSet::new());

    let (xml_impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    quote!(
        impl #xml_impl_generics FromXml<'xml> for #ident #ty_generics #where_clause {
            #[inline]
            fn matches(id: ::instant_xml::Id<'_>, field: Option<::instant_xml::Id<'_>>) -> bool {
                id == ::instant_xml::Id { ns: #default_namespace, name: #name }
            }

            fn deserialize<'cx>(
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut ::instant_xml::Deserializer<'cx, 'xml>,
            ) -> Result<(), ::instant_xml::Error> {
                deserializer.ignore()?;
                *into = Some(Self);
                Ok(())
            }

            type Accumulator = Option<Self>;
            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element;
        }
    )
}

fn is_cow(ty: &syn::Type, elem: fn(&syn::Type) -> bool) -> bool {
    let path = match ungroup(ty) {
        syn::Type::Path(ty) => &ty.path,
        _ => {
            return false;
        }
    };

    let seg = match path.segments.last() {
        Some(seg) => seg,
        None => {
            return false;
        }
    };

    let args = match &seg.arguments {
        syn::PathArguments::AngleBracketed(bracketed) => &bracketed.args,
        _ => {
            return false;
        }
    };

    seg.ident == "Cow"
        && args.len() == 2
        && match (&args[0], &args[1]) {
            (syn::GenericArgument::Lifetime(_), syn::GenericArgument::Type(arg)) => elem(arg),
            _ => false,
        }
}

fn is_str(ty: &syn::Type) -> bool {
    is_primitive_type(ty, "str")
}

fn is_slice_u8(ty: &syn::Type) -> bool {
    match ungroup(ty) {
        syn::Type::Slice(ty) => is_primitive_type(&ty.elem, "u8"),
        _ => false,
    }
}

fn is_primitive_type(ty: &syn::Type, primitive: &str) -> bool {
    match ungroup(ty) {
        syn::Type::Path(ty) => ty.qself.is_none() && is_primitive_path(&ty.path, primitive),
        _ => false,
    }
}

fn is_primitive_path(path: &syn::Path, primitive: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments[0].ident == primitive
        && path.segments[0].arguments.is_empty()
}

pub fn ungroup(mut ty: &syn::Type) -> &syn::Type {
    while let syn::Type::Group(group) = ty {
        ty = &group.elem;
    }
    ty
}

#[derive(Default)]
struct Tokens {
    r#enum: TokenStream,
    branches: TokenStream,
    r#match: TokenStream,
}
